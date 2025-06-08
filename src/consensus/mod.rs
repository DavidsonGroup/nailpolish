/// Consensus calling implementation for duplicate read groups
use crate::cli::ConsensusArgs;
use crate::io::index::{ArchivedDuplicateGroup, FileIndexPath, IndexReader};

use anyhow::{Context as _, Result};
use core::str;
use itertools::Itertools;
use needletail::parser::FastqReader;
use needletail::FastxReader as _;
use std::fmt::Write as StrWrite;
use std::io::{Cursor, Write as IoWrite};

use rayon::prelude::*;
use spoa::{AlignmentEngine, AlignmentType};

mod cluster;
mod formatter;

/// Generate consensus sequences from duplicate read groups
pub fn consensus(args: &crate::cli::ConsensusArgs) -> Result<()> {
    let paths = FileIndexPath::new(&args.input);
    let index_rdr = IndexReader::new(&paths)?;

    let is_multithreaded = args.threads != 1;

    info!(
        "Consensus calling {} → {}",
        paths.fastq().display(),
        args.output
            .as_ref()
            .map_or("stdout".to_string(), |v| v.display().to_string())
    );

    let index = index_rdr.load()?;

    let total_num_reads = index.get_hashmap().len();

    // allocate a thread pool
    if is_multithreaded {
        info!("Using thread pool with {} threads", args.threads,)
    } else {
        info!("Using single thread")
    }

    if is_multithreaded {
        // set the number of threads that Rayon will use
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global()?;
    }

    let mut accessor = index.get_read_accessor(&paths)?;
    let buffer_size: usize = 100usize * args.threads;

    let mut writer = crate::utils::get_writer(args.output.as_deref())?;

    let mut processed_reads = 0;
    let mut processed_clusters = 0;
    let mut processed_duplicate_groups = 0;
    let mut max_clusters = 0;

    let report_progress = |processed_reads: usize,
                           total_num_reads: usize,
                           processed_duplicate_reads: usize,
                           processed_clusters: usize,
                           max_clusters: usize| {
        let cluster_ratio = processed_clusters as f64 / processed_duplicate_reads as f64;

        info!("proc: {processed_reads} / {total_num_reads} groups\t(clusters per duplicate group: avg {cluster_ratio:.2}, max {max_clusters})");
    };

    const REPORT_INTERVAL: usize = 100000; // number of reads to process before reporting progress

    for chunk in &index.groups().chunks(buffer_size) {
        // perform the read operations
        let input = chunk
            .into_iter()
            .map(|group| -> Result<_> {
                let reads = accessor.fetch_reads_archived(group.reads.as_slice())?;
                debug!("length: {}", reads.len());
                Ok((group, reads))
            })
            .collect::<Result<Vec<_>>>()?;

        // perform the consensus call
        let output = if is_multithreaded {
            // parallel: use rayon thread pool
            input
                .par_iter()
                .map(|(g, v)| consensus_call(g, v, args))
                .collect::<Result<Vec<_>>>()?
        } else {
            // otherwise, just use a single iterator
            input
                .iter()
                .map(|(g, v)| consensus_call(g, v, args))
                .collect::<Result<Vec<_>>>()?
        };

        for (elem, clusters, is_duplicate) in output.into_iter() {
            processed_reads += 1;
            if is_duplicate {
                processed_clusters += clusters;
            }
            processed_duplicate_groups += is_duplicate as usize;

            max_clusters = std::cmp::max(max_clusters, clusters);

            if processed_reads % REPORT_INTERVAL == 0 {
                report_progress(
                    processed_reads,
                    total_num_reads,
                    processed_duplicate_groups,
                    processed_clusters,
                    max_clusters,
                );
            }

            write!(writer, "{}", elem)?;
        }
    }

    report_progress(
        processed_reads,
        total_num_reads,
        processed_duplicate_groups,
        processed_clusters,
        max_clusters,
    );

    info!("Complete\ninput: {total_num_reads} reads\nduplicate groups: {processed_duplicate_groups} groups\ntotal clusters: {processed_clusters}");

    Ok(())
}

/// Generates a consensus sequence for a group of reads, or returns the single read if no duplicates exist
fn consensus_call(
    group: &ArchivedDuplicateGroup,
    reads_u8: &Vec<u8>,
    args: &ConsensusArgs,
) -> Result<(String, usize, bool)> {
    let mut header_builder = formatter::HeaderFormatter::new(group, args);
    let read_count = group.reads.len();

    let mut reader = FastqReader::new(Cursor::new(reads_u8));
    let mut result = String::new();

    if read_count == 1 {
        // simplex read
        let read = reader
            .next()
            .context("No read found")?
            .context("Invalid read")?;

        header_builder.add_read(0, &read);

        let seq = read.seq();
        let qual = read.qual().context("No quality")?;

        let header = header_builder.make_consensus_header(0);

        writeln!(
            result,
            "@{}\n{}\n+\n{}",
            header,
            str::from_utf8(&seq)?,
            str::from_utf8(&qual)?
        )?;

        Ok((result, 1, false))
    } else {
        // consensus call

        // initialise `spoa` machinery
        let mut alignment_engine = AlignmentEngine::new(AlignmentType::kOV, 5, -4, -8, -6, -10, -4);

        let mut read_idx = 0usize;

        let mut graphs = vec![spoa::Graph::new()];
        let mut first_read_in_group = true;

        while let Some(read) = reader.next() {
            let read = read.context("Invalid read")?;

            let seq = read.seq();
            let qual = read.qual().context("No quality")?;

            // let mut alignment_result = None;
            let mut alignment_predictions = Vec::new();

            let mut did_cluster = false;
            let mut inserted_cluster_id = 0;
            for (cluster_id, graph) in graphs.iter_mut().enumerate() {
                // Align to the graph
                let align = alignment_engine.align_from_bytes(&seq, graph);

                let will_cluster = if first_read_in_group || args.no_clustering {
                    true
                } else {
                    let alignment_prediction = graph.predict_alignment_from_bytes(&align, &seq);

                    let will_cluster = cluster::should_cluster(&alignment_prediction);
                    alignment_predictions.push(alignment_prediction);

                    debug!("Prediction:\t{alignment_prediction:?}");
                    will_cluster
                };

                if will_cluster {
                    let alignment_result = graph.add_alignment_from_bytes(&align, &seq, &qual);

                    // add each read in the duplicate group to the graph
                    inserted_cluster_id = cluster_id;
                    did_cluster = true;

                    debug!("Added result:\t{alignment_result:?}");

                    // don't need to check any more graphs, we are done
                    break;
                }
            }

            // do we need to add a new graph, because this read didn't cluster?
            if !did_cluster {
                let mut new_graph = spoa::Graph::new();
                let align = alignment_engine.align_from_bytes(&seq, &new_graph);
                alignment_predictions.push(new_graph.add_alignment_from_bytes(&align, &seq, &qual));

                debug!("Added new graph");
                graphs.push(new_graph);
                inserted_cluster_id = graphs.len() - 1;
            }

            // should we report the original reads first?

            header_builder.add_read(inserted_cluster_id, &read);
            if args.report_original_reads {
                let header = header_builder.make_original_header(
                    &read,
                    read_idx,
                    alignment_predictions,
                    inserted_cluster_id,
                );

                writeln!(
                    result,
                    "@{}\n{}\n+\n{}",
                    header,
                    str::from_utf8(&seq).unwrap(),
                    str::from_utf8(&qual).unwrap()
                )?;
            }

            read_idx += 1;
            first_read_in_group = false;
        }

        for (idx, graph) in graphs.iter_mut().enumerate() {
            // Create a consensus read
            let consensus = graph.consensus_with_quality();

            let seq = &consensus.sequence;
            let qual = &consensus.quality;

            let header = header_builder.make_consensus_header(idx);

            writeln!(result, "@{}\n{}\n+\n{}", header, seq, qual)?;
        }

        Ok((result, graphs.len(), true))
    }
}
