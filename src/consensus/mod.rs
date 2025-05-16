/// Consensus calling implementation for duplicate read groups
use crate::cli::ConsensusArgs;
use crate::io::index::{ArchivedDuplicateGroup, FileIndexPath, IndexReader};

use anyhow::{Context as _, Result};
use core::str;
use itertools::Itertools;
use needletail::parser::FastqReader;
use needletail::FastxReader as _;
use std::fmt::Write as StrWrite;
use std::fs::File;
use std::io::{BufWriter, Cursor, Write as IoWrite};
use std::time::Instant;

use rayon::prelude::*;
use spoa::{AlignmentEngine, AlignmentType};

mod formatter;

/// Generate consensus sequences from duplicate read groups
pub fn consensus(args: &crate::cli::ConsensusArgs) -> Result<()> {
    let paths = FileIndexPath::new(&args.input);
    let index_rdr = IndexReader::new(&paths)?;

    info!(
        "Consensus calling {} → {}",
        paths.fastq().display(),
        args.output
            .as_ref()
            .map_or("stdout".to_string(), |v| v.display().to_string())
    );

    let index = index_rdr.load()?;

    // allocate a thread pool
    info!(
        "Creating thread pool with {0} threads + 1 IO thread",
        args.threads
    );

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads + 1)
        .build_global()?;

    let mut accessor = index.get_read_accessor(&paths)?;
    let buffer_size: usize = 500usize * args.threads;

    let mut writer = crate::utils::get_writer(args.output.as_deref())?;

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
            .map(|(g, v)| consensus_call(g, v, args))
            .collect::<Result<Vec<_>>>()?;

        for elem in output.iter() {
            writeln!(writer, "{}", elem)?;
        }
    }

    info!("Finished");

    Ok(())
}

/// Generates a consensus sequence for a group of reads, or returns the single read if no duplicates exist
fn consensus_call(
    group: &ArchivedDuplicateGroup,
    reads_u8: &Vec<u8>,
    args: &ConsensusArgs,
) -> Result<String> {
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

        header_builder.add_read(&read);

        let seq = read.seq();
        let qual = read.qual().context("No quality")?;

        let header = header_builder.finalize();

        write!(
            result,
            "@{}\n{}\n+\n{}",
            header,
            str::from_utf8(&seq)?,
            str::from_utf8(&qual)?
        )?;
    } else {
        // consensus call

        // initialise `spoa` machinery
        let mut alignment_engine = AlignmentEngine::new(AlignmentType::kOV, 5, -4, -8, -6, -10, -4);
        let mut poa_graph = spoa::Graph::new();

        let mut idx = 0usize;
        while let Some(read) = reader.next() {
            let read = read.context("Invalid read")?;

            let seq = read.seq();
            let qual = read.qual().context("No quality")?;

            // add each read in the duplicate group to the graph
            header_builder.add_read(&read);

            // Align to the graph
            let align = alignment_engine.align_from_bytes(&seq, &poa_graph);
            let alignment_result = poa_graph.add_alignment_from_bytes(&align, &seq, &qual);

            debug!("{alignment_result:?}");

            // should we report the original reads first?
            if args.report_original_reads {
                let header = header_builder.make_original_header(&read, idx, &alignment_result);

                writeln!(
                    result,
                    "@{}\n{}\n+\n{}",
                    header,
                    str::from_utf8(&seq).unwrap(),
                    str::from_utf8(&qual).unwrap()
                )?;
            }

            idx += 1;
        }

        // Create a consensus read
        let consensus = poa_graph.consensus_with_quality();

        let seq = &consensus.sequence;
        let qual = &consensus.quality;

        let header = header_builder.finalize();

        write!(result, "@{}\n{}\n+\n{}", header, seq, qual)?;
    }

    Ok(result)
}
