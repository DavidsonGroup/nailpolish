// Copyright 2026 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::path::Path;

use anyhow::{ensure, Result};

use crate::io::index::{DuplicateGroupLocation, FileIndexPath, ReadLocation};
use crate::io::reads::backends::{gzip::GzipSource, plain::PlainSource, ByteRange, Source};

/// Parallel random-access reader. The underlying source owns a dedicated thread pool,
/// distinct from the rayon pool used for consensus calling, so core count can be tuned for
/// I/O throughput rather than computational efficiency.
pub struct RandomAccessReader {
    src: Box<dyn Source>,
}

impl RandomAccessReader {
    pub fn new(path: &Path, compute_threads: usize) -> Result<Self> {
        let src: Box<dyn Source> = if crate::utils::is_gzip_file(path) {
            let index_path = FileIndexPath::new(path).gzip_index();
            Box::new(GzipSource::new(path, index_path, compute_threads.max(1))?)
        } else {
            // No need for compute_threads here; PlainSource chooses a high thread
            // count irregardless of compute threads available since it is I/O bound.
            Box::new(PlainSource::new(path)?)
        };

        Ok(Self { src })
    }

    /// Fetches every read in every group as arrays of bytes
    pub fn fetch_groups<'a>(
        &self,
        groups: &'a [DuplicateGroupLocation],
    ) -> Result<Vec<(&'a DuplicateGroupLocation, Vec<Vec<u8>>)>> {
        // NOTE: this is highly order-dependent. `read_locs` should be in the same
        // order as `groups`
        let read_locs = groups
            .iter()
            .flat_map(|g| g.reads.iter())
            .collect::<Vec<_>>();
        let reads = self.fetch_reads(&read_locs)?;

        ensure!(
            reads.len() == read_locs.len(),
            "Mismatch between reads and read locations"
        );

        // create output vector
        let mut read_iter = reads.into_iter();
        let output = groups
            .iter()
            .map(|g| {
                (
                    g,
                    read_iter
                        .by_ref()
                        .take(g.reads.len())
                        .collect::<Vec<Vec<u8>>>(),
                )
            })
            .collect();

        Ok(output)
    }

    /// Fetches a flat list of reads as arrays of bytes, preserving input order.
    pub fn fetch_reads(&self, reads: &[&ReadLocation]) -> Result<Vec<Vec<u8>>> {
        let mut tagged_reads: Vec<_> = reads.into_iter().enumerate().collect();

        // sort by offset, for efficient sequential access within chunks
        // eliminates reseek/decompression if reads are near-sequential
        tagged_reads.sort_unstable_by_key(|(_, r)| r.pos());

        let ranges: Vec<ByteRange> = tagged_reads
            .iter()
            .map(|(_, r)| ByteRange {
                start: r.pos(),
                end: r.pos() + r.byte_len() as u64,
            })
            .collect();

        // the backend handles parallelisation; results come back in request order
        let data = self.src.fetch_byte_ranges(&ranges)?;

        ensure!(
            data.len() == tagged_reads.len(),
            "Mismatch between fetched byte ranges and read locations"
        );

        // re-sort by original input order
        let mut tagged_output: Vec<(usize, Vec<u8>)> = tagged_reads
            .into_iter()
            .map(|(idx, _)| idx)
            .zip(data)
            .collect();
        tagged_output.sort_unstable_by_key(|(idx, _)| *idx);
        let output = tagged_output.into_iter().map(|(_, data)| data).collect();

        Ok(output)
    }
}
