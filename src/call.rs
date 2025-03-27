use crate::duplicates::DuplicateMap;
use crate::io::{ReadType, Record, UMIGroup, UMIGroupCollection};

use spoa::{AlignmentEngine, AlignmentType};

use rayon::prelude::*;

use std::io::prelude::*;

use crate::index::IndexReader;
use anyhow::Result;

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
pub fn consensus(
    collection: &mut UMIGroupCollection,
    writer: &mut impl Write,
    threads: usize,
    duplicates_only: bool,
    output_originals: bool,
) -> Result<()> {
    info!("Creating thread pool with {threads} threads");

    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()?;

    let mut duplicate_iterator = collection.stream_iter(duplicates_only);

    let chunk_size = 5000usize * threads;

    // this vector stores the indexes of each group within the buf_duplicates and buf_single buffers
    let mut buf_locations = Vec::with_capacity(chunk_size);
    let mut buf_duplicates = Vec::new();
    let mut buf_single = Vec::new();

    let mut idx = 0;
    let mut first = true;

    let mut end_of_buffer = false;
    loop {
        if let Some(group) = duplicate_iterator.next()? {
            idx += 1;

            if (idx > 0) && (idx % 100000 == 0) {
                info!("Called {} reads...", idx);
            }

            let single = group.records.len() == 1;
            if (single && !duplicates_only) || group.ignore {
                buf_locations.push(GroupType::Simplex(buf_single.len()));
                buf_single.push(group)
            } else {
                buf_locations.push(GroupType::Duplex(buf_duplicates.len()));
                buf_duplicates.push(group);
            }

            end_of_buffer = false;
        } else {
            end_of_buffer = true;
        };

        // if we have filled the buffer OR are at the end, process this chunk
        if (buf_locations.len() == chunk_size) || end_of_buffer {
            // single records are not multithreaded to save on IPC costs;
            // use rayon to multithread duplicate buffer record calling
            buf_single.iter_mut().for_each(call_umi_group);
            buf_duplicates.par_iter_mut().for_each(call_umi_group);

            for (pos, loc) in buf_locations.iter().enumerate() {
                let group = match loc {
                    GroupType::Simplex(i) => buf_single.get_mut(*i),
                    GroupType::Duplex(i) => buf_duplicates.get_mut(*i),
                }
                .expect("Index is invalid; should not occur");

                // output original reads as well, if requested
                if matches!(loc, GroupType::Duplex(_)) && output_originals {
                    if !first {
                        writer.write_all(b"\n")?;
                    }
                    first = false;

                    let group_size = group.records.len();
                    for (idx, r) in group.records.iter_mut().enumerate() {
                        r.add_metadata(
                            group.index,
                            ReadType::Original,
                            idx + 1,
                            group_size,
                            group.avg_qual,
                        );
                        r.write_fastq(&mut *writer)?;
                    }
                }

                // add a newline at the start, unless this is the first line in the file
                if !first {
                    writer.write_all(b"\n")?;
                }
                first = false;

                let rec = group.consensus.as_mut().expect("Should never be None");
                rec.write_fastq(&mut *writer)?;
            }

            // empty the buffer
            buf_single.clear();
            buf_duplicates.clear();
            buf_locations.clear();
        }

        // if we are at the end, let's go
        if end_of_buffer {
            break;
        }
    }

    Ok(())
}

/// Generates a consensus sequence from a group of reads.
///
/// # Arguments
///
/// * `group` - A `UMIGroup` containing the reads to be processed.
/// * `output_originals` - A boolean indicating whether to include the original reads in the
///   output alongside the consensus read.
///
/// # Returns
///
/// A `String` containing the consensus sequence in FASTQ format.
fn call_umi_group(group: &mut UMIGroup) {
    let length = group.records.len();

    // // process ignored reads first

    // TODO: why is this commented out?
    // if group.ignore {
    //     for record in group.records.iter() {
    //         io::write_read(&mut output, record, &group, ReadType::Ignored, false).unwrap();
    //     }
    //     return output.into_inner();
    // }

    // for singletons, the read is its own consensus
    if length == 1 {
        let mut rec = group.records[0].clone();

        rec.add_metadata(group.index, ReadType::Single, 1, 1, group.avg_qual);

        group.consensus = Some(rec);

        return;
    }

    // initialise `spoa` machinery
    let mut alignment_engine = AlignmentEngine::new(AlignmentType::kOV, 5, -4, -8, -6, -10, -4);
    let mut poa_graph = spoa::Graph::new();

    // add each read in the duplicate group to the graph
    for record in group.records.iter() {
        // TODO: align originals and output as well

        // Align to the graph
        let align = alignment_engine.align_from_bytes(record.seq.as_ref(), &poa_graph);
        poa_graph.add_alignment_from_bytes(&align, record.seq.as_ref(), record.qual.as_ref());
    }

    // Create a consensus read
    let consensus = poa_graph.consensus_with_quality();
    let mut rec = Record {
        id: group.id.to_string(),
        seq: consensus.sequence,
        qual: consensus.quality,
    };

    rec.add_metadata(
        group.index,
        ReadType::Consensus,
        0,
        group.records.len(),
        group.avg_qual,
    );

    group.consensus = Some(rec);
}
