/// Consensus calling implementation for duplicate read groups
use crate::cli::ConsensusArgs;
use crate::io::index::{ArchivedDuplicateGroup, FileIndexPath, IndexReader};

use anyhow::Result;
use itertools::Itertools;
use std::fmt::Write as StrWrite;
use std::fs::File;
use std::io::Write as IoWrite;
use std::time::Instant;

use bio::io::fastq::Record;
use rayon::prelude::*;
use spoa::{AlignmentEngine, AlignmentType};

mod formatter;

/// Generate consensus sequences from duplicate read groups
pub fn consensus(cli: &crate::cli::ConsensusArgs) -> Result<()> {
    let paths = FileIndexPath::new(&cli.input);
    let index_rdr = IndexReader::new(&paths)?;

    info!(
        "Consensus calling {} → {}",
        paths.fastq().display(),
        cli.output.display()
    );

    let index = index_rdr.load()?;

    // allocate a thread pool
    info!(
        "Creating thread pool with {0} threads + 1 IO thread",
        cli.threads
    );

    rayon::ThreadPoolBuilder::new()
        .num_threads(cli.threads + 1)
        .build_global()?;

    let mut accessor = index.get_read_accessor(&paths)?;
    let buffer_size: usize = 500usize * cli.threads;

    let mut file_w = File::create_new(cli.output.clone())?;

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
            .map(|(g, v)| consensus_call(g, v, cli))
            .collect::<Result<Vec<_>>>()?;

        for elem in output.iter() {
            writeln!(file_w, "{}", elem)?;
        }
    }

    Ok(())
}

/// Generates a consensus sequence for a group of reads, or returns the single read if no duplicates exist
fn consensus_call(
    group: &ArchivedDuplicateGroup,
    reads: &Vec<Record>,
    args: &ConsensusArgs,
) -> Result<String> {
    let mut header = formatter::make_consensus_header(group, reads, args);

    let mut result = String::new();

    if reads.len() == 1 {
        // simplex read
        let read = &reads[0];

        let seq = unsafe { std::str::from_utf8_unchecked(read.seq()) };
        let qual = unsafe { std::str::from_utf8_unchecked(read.qual()) };

        write!(result, "@{}\n{}\n+\n{}", header, seq, qual)?;
    } else {
        // consensus call
        let start_time = Instant::now();

        // initialise `spoa` machinery
        let mut alignment_engine = AlignmentEngine::new(AlignmentType::kOV, 5, -4, -8, -6, -10, -4);
        let mut poa_graph = spoa::Graph::new();

        // add each read in the duplicate group to the graph
        for (idx, record) in reads.iter().enumerate() {
            // Align to the graph
            let align = alignment_engine.align_from_bytes(record.seq(), &poa_graph);
            let alignment_result =
                poa_graph.add_alignment_from_bytes(&align, record.seq(), record.qual());

            debug!("{alignment_result:?}");

            // should we report the original reads first?
            if args.report_original_reads {
                let header =
                    formatter::make_original_header(group, record, idx, &alignment_result, args);

                let seq = std::str::from_utf8(record.seq())?;
                let qual = std::str::from_utf8(record.qual())?;

                writeln!(result, "@{}\n{}\n+\n{}", header, seq, qual)?;
            }
        }

        // Create a consensus read
        let consensus = poa_graph.consensus_with_quality();

        if args.extra_stats {
            write!(header, "|elapsed_us={}", start_time.elapsed().as_micros())?;
        }

        let seq = &consensus.sequence;
        let qual = &consensus.quality;

        write!(result, "@{}\n{}\n+\n{}", header, seq, qual)?;
    }

    Ok(result)
}
