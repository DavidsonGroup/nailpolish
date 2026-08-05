// Copyright 2026 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::cell::RefCell;
use std::path::Path;

use anyhow::{ensure, Context, Result};
use rayon::prelude::*;

use crate::io::index::{DuplicateGroupLocation, FileIndexPath, ReadLocation};
use crate::io::reads::backends::{gzip::GzipSource, plain::PlainSource, Accessor, Source};

thread_local! {
    static ACCESSOR: RefCell<Option<Box<dyn Accessor>>> = const { RefCell::new(None) };
}

/// Access the underlying accessor for the given source, creating it if necessary. This is used to
/// ensure that the accessor is only created once per thread, and that it is reused for all
/// subsequent read requests.
fn with_accessor<T>(src: &Box<dyn Source>, f: impl FnOnce(&mut dyn Accessor) -> T) -> T {
    ACCESSOR.with(|cell| {
        let mut cell = cell.borrow_mut();
        if cell.is_none() {
            *cell = Some(
                // if there is an error here, we should throw; not expected.
                src.make()
                    .expect("Could not make accessor for random access reader"),
            );
        }
        let accessor = cell.as_mut().unwrap();

        f(accessor.as_mut())
    })
}

/// Parallel random-access reader. Owns a dedicated thread pool, distinct from
/// the rayon pool used for consensus calling, so core count can be tuned for I/O throughput
/// rather than computational efficiency.
pub struct RandomAccessReader {
    src: Box<dyn Source>,
    pool: rayon::ThreadPool,
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

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(src.threads())
            .thread_name(|i| format!("nailpolish-io-{i}"))
            .build()
            .context("Failed to build I/O thread pool")?;

        Ok(Self { src, pool })
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

        // read the optimal chunk size from the source
        let chunk_size = self.src.optimal_chunk_size(reads.len());

        // create output buffer with slots for each read
        let mut tagged_output: Vec<(usize, Vec<u8>)> = vec![(0, Vec::new()); reads.len()];

        // parallelize the read requests in chunks, each chunk serviced by a single thread
        // Rayon handles work-stealing and load balancing across threads
        self.pool.install(|| -> Result<()> {
            tagged_output
                .par_chunks_mut(chunk_size)
                .zip(tagged_reads.par_chunks(chunk_size))
                .try_for_each(|(out_chunk, read_chunk)| -> Result<()> {
                    with_accessor(&self.src, |accessor| {
                        for (out_slot, (read_idx, read)) in out_chunk.iter_mut().zip(read_chunk) {
                            let data = accessor.read(read.pos(), read.byte_len())?;
                            *out_slot = (*read_idx, data);
                        }
                        Ok(())
                    })
                })
        })?;

        // re-sort by original input order
        tagged_output.sort_unstable_by_key(|(idx, _)| *idx);
        let output = tagged_output.into_iter().map(|(_, data)| data).collect();

        Ok(output)
    }
}
