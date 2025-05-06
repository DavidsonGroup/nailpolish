use crate::cli::CallArgs;
use crate::io::index::storage::{FileIndexPath, IndexReader};
use crate::io::index::ArchivedDuplicateGroup;
use anyhow::Result;
use bio::io::fastq::Record;
use formatter::make_consensus_header;
use itertools::Itertools;
use rayon::prelude::*;
use spoa::{AlignmentEngine, AlignmentType};
use std::fmt::Write as StrWrite;
use std::fs::File;
use std::io::Write as IoWrite;
use std::time::Instant;

mod formatter;

enum GroupType {
    Simplex(usize),
    Duplex(usize),
}

/// Generates consensus sequences from the input in a thread-stable manner.
///
/// # Arguments
///
/// * `input` - A string slice that holds the path to the input file.
/// * `writer` - A mutable reference to an object that implements the `Write` trait,
///   used for writing the output.
/// * `duplicates` - A `DuplicateMap` containing the duplicate reads.
/// * `threads` - The number of threads to use for parallel processing.
/// * `duplicates_only` - A boolean indicating whether to process only duplicate reads.
/// * `output_originals` - A boolean indicating whether to include the original reads in the output.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if successful, or an error if an error occurs
///   during processing.
pub fn consensus(cli: &crate::cli::CallArgs) -> Result<()> {
    let paths = FileIndexPath::new(&cli.input);
    let index_rdr = IndexReader::new(paths)?;

    let index = index_rdr.load()?;

    // allocate a thread pool
    info!(
        "Creating thread pool with {0} threads + 1 IO thread",
        cli.threads
    );

    rayon::ThreadPoolBuilder::new()
        .num_threads(cli.threads + 1)
        .build_global()?;

    let mut accessor = index.get_read_accessor()?;
    let buffer_size: usize = 500usize * cli.threads;

    let mut file_w = File::create(cli.output.clone())?;
    let mut first = true;

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

        // perform the parallel consensus call
        let output: Vec<String> = input
            .par_iter()
            .map(|(g, v)| consensus_call(g, v, &cli))
            .collect();

        // perform the write operations
        if first {
            first = false
        } else {
            file_w.write_all(b"\n")?;
        }

        let output_str = output.join("\n");
        file_w.write_all(output_str.as_bytes())?;
    }

    Ok(())
}

fn consensus_call(group: &ArchivedDuplicateGroup, reads: &Vec<Record>, args: &CallArgs) -> String {
    let mut header = make_consensus_header(group, reads, args);

    if reads.len() == 1 {
        // simplex read
        let read = &reads[0];

        let seq = unsafe { std::str::from_utf8_unchecked(read.seq()) };
        let qual = unsafe { std::str::from_utf8_unchecked(read.qual()) };

        format!("@{}\n{}\n+\n{}", header, seq, qual)
    } else {
        // consensus call
        let start_time = Instant::now();

        // initialise `spoa` machinery
        let mut alignment_engine = AlignmentEngine::new(AlignmentType::kOV, 5, -4, -8, -6, -10, -4);
        let mut poa_graph = spoa::Graph::new();

        // add each read in the duplicate group to the graph
        for record in reads.iter() {
            // TODO: align originals and output as well

            // Align to the graph
            let align = alignment_engine.align_from_bytes(record.seq(), &poa_graph);
            poa_graph.add_alignment_from_bytes(&align, record.seq(), record.qual());
        }

        // Create a consensus read
        let consensus = poa_graph.consensus_with_quality();

        if args.debugging_header {
            write!(header, "|elapsed_us={}", start_time.elapsed().as_micros()).unwrap();
        }

        let seq = &consensus.sequence;
        let qual = &consensus.quality;

        format!("@{}\n{}\n+\n{}", header, seq, qual)
    }
}
