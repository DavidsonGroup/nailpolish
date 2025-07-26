// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use core::str;
use std::cell::RefCell;
use std::fmt::Write as StrWrite;
use std::io::{Cursor, Write as IoWrite};

use anyhow::{Context as _, Result};
use itertools::Itertools;
use needletail::parser::{FastqReader, SequenceRecord};
use needletail::FastxReader as _;
use rayon::prelude::*;
use rkyv::option::ArchivedOption;
use rkyv::string::ArchivedString;
use rkyv::vec::ArchivedVec;
use spoa::{AlignmentEngine, AlignmentType};

use crate::cli::ConsensusArgs;
use crate::io::index::filter::{filter_group_locations, FilterOpts};
use crate::io::index::{DuplicateGroup, DuplicateGroupType, FileIndexPath, IndexReader};

mod cluster;
mod formatter;
mod output_writer;

type ArchivedCaptures = ArchivedVec<ArchivedOption<ArchivedString>>;

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

    let mut writer = output_writer::OutputWriter::new(
        crate::utils::get_writer(cli.output.as_deref())?,
        index.metadata().total_reads,
        cli.no_clustering,
    );

    let opts = FilterOpts::new(cli);
    let captures = index.captures();

    let buffer_chunk_size = 1024 * cli.threads; // number of groups to multithread at one time
    for chunk in &index.groups().chunks(buffer_chunk_size) {
        let chunk_groups = chunk
            .into_iter()
            .map(|group_loc| -> Result<Vec<_>> {
                let reads = accessor.fetch_group(&group_loc)?;
                let groups = filter_group_locations(&group_loc, reads, &opts);
                Ok(groups)
            })
            .flatten_ok()
            .collect::<Result<Vec<_>>>()?;

        process_groups_parallel(&chunk_groups, cli, captures, &mut writer)?;
    }

    writer.finalize()
}

fn process_groups_parallel(
    groups: &Vec<DuplicateGroup>,
    args: &ConsensusArgs,
    captures: &ArchivedCaptures,
    writer: &mut output_writer::OutputWriter<impl IoWrite>,
) -> Result<()> {
    let closure = |g: &DuplicateGroup| {
        if g.group_type == DuplicateGroupType::Filtered {
            handle_filtered_reads(g, args, captures)
        } else if g.reads.len() == 1 {
            process_simplex_read(g, args, captures)
        } else {
            process_consensus_reads(g, args, captures)
        }
    };

    let output = if args.threads == 1 {
        groups.iter().map(closure).collect::<Result<Vec<_>>>()?
    } else {
        groups.par_iter().map(closure).collect::<Result<Vec<_>>>()?
    };

    for (content, metadata) in output {
        writer.write_read(&content, &metadata)?;
    }

    Ok(())
}

/// Filtered read caller: output filtered groups
fn handle_filtered_reads(
    group: &DuplicateGroup,
    args: &ConsensusArgs,
    captures: &ArchivedCaptures,
) -> Result<(String, output_writer::OutputMetadata)> {
    let reads_u8 = group.reads.concat();
    let header_builder = formatter::HeaderFormatter::new(group, args, captures);
    let mut reader = FastqReader::new(Cursor::new(reads_u8));
    let mut result = String::new();

    let mut read_idx = 1usize;

    while let Some(read) = reader.next() {
        let read = read.context("Invalid read")?;
        let seq = read.seq();
        let qual = read.qual().context("No quality")?;

        let header = header_builder.make_filtered_header(read_idx);

        writeln!(
            result,
            "@{}\n{}\n+\n{}",
            header,
            str::from_utf8(&seq).unwrap(),
            str::from_utf8(qual).unwrap()
        )?;

        read_idx += 1;
    }

    Ok((
        result,
        output_writer::OutputMetadata {
            num_reads: group.reads.len(),
            clusters: 0,
            num_filtered_reads: group.reads.len(),
            is_duplicate: true,
        },
    ))
}

/// Simplex read caller: processes single reads without consensus calling
fn process_simplex_read(
    group: &DuplicateGroup,
    args: &ConsensusArgs,
    captures: &ArchivedCaptures,
) -> Result<(String, output_writer::OutputMetadata)> {
    let reads_u8 = group.reads.concat();
    let header_builder = formatter::HeaderFormatter::new(group, args, captures);

    let mut reader = FastqReader::new(Cursor::new(reads_u8));

    let read = reader
        .next()
        .context("No read found")?
        .context("Invalid read")?;

    let header = header_builder.make_simplex_header(String::from_utf8(read.id().to_vec()).unwrap());

    let seq = read.seq();
    let qual = read.qual().context("No quality")?;

    let result = format!(
        "@{}\n{}\n+\n{}\n",
        header,
        str::from_utf8(&seq)?,
        str::from_utf8(qual)?
    );

    Ok((
        result,
        output_writer::OutputMetadata {
            num_reads: 1,
            clusters: 1,
            num_filtered_reads: 0,
            is_duplicate: false,
        },
    ))
}

pub struct Cluster {
    pub output: String,
    pub orig_read_headers: Vec<String>,
    pub graph: spoa::Graph,
    pub read_count: usize,
    pub id: usize,
}

/// Handles multi-read consensus calling with clustering
fn process_consensus_reads(
    group: &DuplicateGroup,
    args: &ConsensusArgs,
    captures: &ArchivedCaptures,
) -> Result<(String, output_writer::OutputMetadata)> {
    // Filtered groups should always be simplex reads.
    assert_ne!(group.group_type, DuplicateGroupType::Filtered);

    let reads_u8 = group.reads.concat();
    let header_builder = formatter::HeaderFormatter::new(group, args, captures);
    let mut reader = FastqReader::new(Cursor::new(reads_u8));

    // initialize cluster with just one empty cluster
    let mut clusters = vec![Cluster {
        output: String::new(),
        orig_read_headers: vec![],
        graph: spoa::Graph::new(),
        read_count: 0,
        id: 1,
    }];

    let mut read_idx = 1usize;
    let mut first_read_in_group = true;
    while let Some(read) = reader.next() {
        // fetch read quality and sequence
        let read = read.context("Invalid read")?;

        // cluster this read
        let (inserted_cluster, alignment_predictions) =
            insert_read_into_clusters(&read, &mut clusters, args, first_read_in_group)?;
        inserted_cluster.read_count += 1;

        // save the read header, if needed
        // we just leave the vec empty if the argument was not passed
        // to avoid unnecessary complexity
        if args.report_original_header {
            inserted_cluster
                .orig_read_headers
                .push(String::from_utf8(read.id().to_vec()).unwrap());
        }

        // report the read, if needed
        if args.report_original_reads {
            let header = header_builder.make_original_header(
                read_idx,
                alignment_predictions,
                inserted_cluster,
            );

            writeln!(
                inserted_cluster.output,
                "@{}\n{}\n+\n{}",
                header,
                str::from_utf8(&read.seq()).unwrap(),
                str::from_utf8(read.qual().unwrap()).unwrap(),
            )?;
        }

        read_idx += 1;
        first_read_in_group = false;
    }

    let result = generate_consensus_output(&mut clusters, &header_builder)?;

    Ok((
        result,
        output_writer::OutputMetadata {
            num_reads: group.reads.len(),
            clusters: clusters.len(),
            num_filtered_reads: 0,
            is_duplicate: true,
        },
    ))
}

/// Determines which graph to cluster read with or creates new graph
fn insert_read_into_clusters<'a>(
    record: &SequenceRecord,
    clusters: &'a mut Vec<Cluster>,
    args: &ConsensusArgs,
    first_read_in_group: bool,
) -> Result<(&'a mut Cluster, Vec<spoa::AlignmentResult>)> {
    // we will store individual predictions here
    let mut alignment_predictions = Vec::new();

    let mut did_cluster = false;
    let mut inserted_cluster_id = 0;

    // extract sequence and quality from the read
    let seq = &record.seq();
    let qual = record
        .qual()
        .context("Only .fastq files are supported for now")?;

    for (cluster_id, cluster) in clusters.iter_mut().enumerate() {
        let graph = &mut cluster.graph;

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
            let alignment_result = graph.add_alignment_from_bytes(&align, seq, &qual);
            inserted_cluster_id = cluster_id;
            did_cluster = true;
            debug!("Added result:\t{alignment_result:?}");
            break;
        }
    }

    // Create new graph if read didn't cluster with existing ones
    if !did_cluster {
        let mut new_cluster = Cluster {
            output: String::new(),
            orig_read_headers: vec![],
            graph: spoa::Graph::new(),
            read_count: 0,
            id: clusters.len() + 1,
        };

        let align =
            with_alignment_engine(|engine| engine.align_from_bytes(seq, &new_cluster.graph));

        new_cluster
            .graph
            .add_alignment_from_bytes(&align, seq, qual);

        debug!("Added new graph");

        clusters.push(new_cluster);
        inserted_cluster_id = clusters.len() - 1;
    }

    Ok((&mut clusters[inserted_cluster_id], alignment_predictions))
}

/// Creates final consensus sequences from graphs
fn generate_consensus_output(
    clusters: &mut Vec<Cluster>,
    header_builder: &formatter::HeaderFormatter,
) -> Result<String> {
    let mut result = String::new();

    for cluster in clusters.iter_mut() {
        let consensus = cluster.graph.consensus_with_quality();
        let seq = &consensus.sequence;
        let qual = &consensus.quality;
        let header = header_builder.make_consensus_header(cluster);

        if cluster.output.len() > 0 {
            write!(result, "{}", cluster.output).unwrap();
        }

        writeln!(result, "@{}\n{}\n+\n{}", header, seq, qual)?;
    }

    Ok(result)
}
