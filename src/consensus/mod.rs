// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

/// Consensus calling implementation for duplicate read groups
use std::cell::RefCell;
use std::fmt::Write as StrWrite;
use std::io::{Cursor, Write as IoWrite};
use std::str;

use anyhow::{Context as _, Result};
use itertools::Itertools;
use needletail::parser::FastqReader;
use needletail::FastxReader as _;
use rayon::prelude::*;
use spoa::{AlignmentEngine, AlignmentType};

use crate::cli::ConsensusArgs;
use crate::io::index::filter::{filter_group_locations, FilterOpts};
use crate::io::index::{DuplicateGroup, DuplicateGroupType, FileIndexPath, IndexReader};

mod cluster;
mod formatter;

// CN: perf-optimization; thread-local AlignmentEngine to avoid creating new engine per consensus_call
thread_local! {
    static ALIGNMENT_ENGINE: RefCell<AlignmentEngine> = RefCell::new(
        AlignmentEngine::new(AlignmentType::kOV, 5, -4, -8, -6, -10, -4)
    );
}

/// Helper function to get access to the thread-local AlignmentEngine
fn with_alignment_engine<F, R>(f: F) -> R
where
    F: FnOnce(&mut AlignmentEngine) -> R,
{
    ALIGNMENT_ENGINE.with(|engine| {
        let mut engine = engine.borrow_mut();
        f(&mut engine)
    })
}

/// Generate consensus sequences from duplicate read groups
pub fn consensus(cli: &crate::cli::ConsensusArgs) -> Result<()> {
    let paths = FileIndexPath::new(&cli.input);
    let index_rdr = IndexReader::new(&paths)?;

    let is_multithreaded = cli.threads != 1;

    info!(
        "Consensus calling {} → {}",
        paths.fastq().display(),
        cli.output
            .as_ref()
            .map_or("stdout".to_string(), |v| v.display().to_string())
    );

    let index = index_rdr.load()?;

    let total_num_reads = index.metadata().total_reads;

    // allocate a thread pool
    if is_multithreaded {
        info!("Using thread pool with {} threads", cli.threads,)
    } else {
        info!("Using single thread")
    }

    if is_multithreaded {
        // set the number of threads that Rayon will use
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.threads)
            .build_global()?;
    }

    let mut accessor = index.get_read_accessor(&paths)?;
    let buffer_size: usize = 100usize * cli.threads;

    let mut writer = crate::utils::get_writer(cli.output.as_deref())?;

    let mut processed_reads = 0;
    let mut processed_clusters = 0;
    let mut processed_duplicate_groups = 0;
    let mut max_clusters = 0;

    let report_progress = |processed_reads: usize,
                           total_num_reads: usize,
                           processed_groups: usize,
                           processed_clusters: usize,
                           max_clusters: usize| {
        let cluster_ratio = processed_clusters as f64 / processed_groups as f64;

        if cli.no_clustering {
            info!("proc: {processed_reads} / {total_num_reads} reads");
        } else {
            info!("proc: {processed_reads} / {total_num_reads} reads\t(clusters per duplicate group: avg {cluster_ratio:.2}, max {max_clusters})");
        }
    };

    const REPORT_INTERVAL: usize = 10000; // number of reads to process before reporting progress

    // CN: group-size-filter; large groups will be split into individual reads via flat_map
    let groups_iter = index.groups();

    for chunk in &groups_iter.chunks(buffer_size) {
        // perform the read operations
        let input = chunk
            .into_iter()
            .map(|group_loc| -> Result<Vec<_>> {
                let opts = FilterOpts::new(cli);
                let reads = accessor.fetch_group(&group_loc)?;
                let groups = filter_group_locations(&group_loc, reads, &opts);

                Ok(groups)
            })
            .flatten_ok()
            .collect::<Result<Vec<_>>>()?;

        // perform the consensus call
        let output = if is_multithreaded {
            // parallel: use rayon thread pool
            input
                .par_iter()
                .map(|group| consensus_call(group, cli))
                .collect::<Result<Vec<_>>>()?
        } else {
            // otherwise, just use a single iterator
            input
                .iter()
                .map(|group| consensus_call(group, cli))
                .collect::<Result<Vec<_>>>()?
        };

        for (elem, num_reads, clusters, is_duplicate) in output.into_iter() {
            processed_reads += num_reads;
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

            writer.flush()?;
            log::logger().flush();
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

/// Main consensus caller: coordinates simplex vs consensus processing
fn consensus_call(
    group: &DuplicateGroup,
    args: &ConsensusArgs,
) -> Result<(String, usize, usize, bool)> {
    if group.group_type == DuplicateGroupType::Filtered {
        handle_filtered_reads(group, args)
    } else if group.reads.len() == 1 {
        handle_simplex_read(group, args)
    } else {
        process_consensus_reads(group, args)
    }
}

/// Filtered read caller: output filtered groups
fn handle_filtered_reads(
    group: &DuplicateGroup,
    args: &ConsensusArgs,
) -> Result<(String, usize, usize, bool)> {
    let reads_u8 = group.reads.concat();
    let header_builder = formatter::HeaderFormatter::new(group, args);
    let mut reader = FastqReader::new(Cursor::new(reads_u8));
    let mut result = String::new();

    let mut read_idx = 0usize;

    while let Some(read) = reader.next() {
        let read = read.context("Invalid read")?;
        let seq = read.seq();
        let qual = read.qual().context("No quality")?;

        let header = header_builder.make_filtered_header(&read, read_idx);

        writeln!(
            result,
            "@{}\n{}\n+\n{}",
            header,
            str::from_utf8(&seq).unwrap(),
            str::from_utf8(qual).unwrap()
        )?;

        read_idx += 1;
    }

    Ok((result, group.reads.len(), 0, false))
}

/// Simplex read caller: processes single reads without consensus calling
fn handle_simplex_read(
    group: &DuplicateGroup,
    args: &ConsensusArgs,
) -> Result<(String, usize, usize, bool)> {
    let reads_u8 = group.reads.concat();
    let mut header_builder = formatter::HeaderFormatter::new(group, args);
    let mut reader = FastqReader::new(Cursor::new(reads_u8));
    let mut result = String::new();

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
        str::from_utf8(qual)?
    )?;

    Ok((result, 1, 1, false))
}

/// Handles multi-read consensus calling with clustering
fn process_consensus_reads(
    group: &DuplicateGroup,
    args: &ConsensusArgs,
) -> Result<(String, usize, usize, bool)> {
    // Filtered groups should always be simplex reads.
    assert_ne!(group.group_type, DuplicateGroupType::Filtered);

    let reads_u8 = group.reads.concat();
    let mut header_builder = formatter::HeaderFormatter::new(group, args);
    let mut reader = FastqReader::new(Cursor::new(reads_u8));
    let mut result = String::new();

    let mut read_idx = 0usize;
    let mut graphs = vec![spoa::Graph::new()];
    let mut first_read_in_group = true;

    while let Some(read) = reader.next() {
        let read = read.context("Invalid read")?;
        let seq = read.seq();
        let qual = read.qual().context("No quality")?;

        let (inserted_cluster_id, alignment_predictions) =
            cluster_read_to_graphs(&seq, qual, &mut graphs, args, first_read_in_group);

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
                str::from_utf8(qual).unwrap()
            )?;
        }

        read_idx += 1;
        first_read_in_group = false;
    }

    let consensus_output = generate_consensus_output(&mut graphs, &header_builder)?;
    result.push_str(&consensus_output);

    Ok((result, group.reads.len(), graphs.len(), true))
}

/// Determines which graph to cluster read with or creates new graph
fn cluster_read_to_graphs(
    seq: &[u8],
    qual: &[u8],
    graphs: &mut Vec<spoa::Graph>,
    args: &ConsensusArgs,
    first_read_in_group: bool,
) -> (usize, Vec<spoa::AlignmentResult>) {
    let mut alignment_predictions = Vec::new();
    let mut did_cluster = false;
    let mut inserted_cluster_id = 0;

    for (cluster_id, graph) in graphs.iter_mut().enumerate() {
        let align = with_alignment_engine(|engine| engine.align_from_bytes(seq, graph));

        let will_cluster = if first_read_in_group || args.no_clustering {
            true
        } else {
            let alignment_prediction = graph.predict_alignment_from_bytes(&align, seq);
            let will_cluster = cluster::should_cluster(&alignment_prediction);
            alignment_predictions.push(alignment_prediction);
            debug!("Prediction:\t{alignment_prediction:?}");
            will_cluster
        };

        if will_cluster {
            let alignment_result = graph.add_alignment_from_bytes(&align, seq, qual);
            inserted_cluster_id = cluster_id;
            did_cluster = true;
            debug!("Added result:\t{alignment_result:?}");
            break;
        }
    }

    // Create new graph if read didn't cluster with existing ones
    if !did_cluster {
        let mut new_graph = spoa::Graph::new();
        let align = with_alignment_engine(|engine| engine.align_from_bytes(seq, &new_graph));
        alignment_predictions.push(new_graph.add_alignment_from_bytes(&align, seq, qual));
        debug!("Added new graph");
        graphs.push(new_graph);
        inserted_cluster_id = graphs.len() - 1;
    }

    (inserted_cluster_id, alignment_predictions)
}

/// Creates final consensus sequences from graphs
fn generate_consensus_output(
    graphs: &mut [spoa::Graph],
    header_builder: &formatter::HeaderFormatter,
) -> Result<String> {
    let mut result = String::new();

    for (idx, graph) in graphs.iter_mut().enumerate() {
        let consensus = graph.consensus_with_quality();
        let seq = &consensus.sequence;
        let qual = &consensus.quality;
        let header = header_builder.make_consensus_header(idx);

        writeln!(result, "@{}\n{}\n+\n{}", header, seq, qual)?;
    }

    Ok(result)
}
