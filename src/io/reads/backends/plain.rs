// Copyright 2026 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

//! Plain (uncompressed) random-access backend.

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use rayon::prelude::*;

use super::{Accessor, ByteRange, Source};

/// Plain files are latency-bound: each read is a network round-trip of a few hundred
/// microseconds, and threads waiting on one are parked, not running. A high thread
/// count gives queue depth against that latency.
const PLAIN_THREADS: usize = 128;

thread_local! {
    static ACCESSOR: RefCell<Option<PlainAccessor>> = const { RefCell::new(None) };
}

/// Access the underlying accessor for the given source, creating it if necessary. This is used to
/// ensure that the accessor is only created once per thread, and that it is reused for all
/// subsequent read requests.
fn with_accessor<T>(src: &PlainSource, f: impl FnOnce(&mut PlainAccessor) -> T) -> T {
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

        f(accessor)
    })
}

pub(crate) struct PlainSource {
    file: Arc<File>,
    pool: rayon::ThreadPool,
}

impl PlainSource {
    pub(crate) fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)
            .with_context(|| format!("Error opening read file {}", path.display()))?;

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(PLAIN_THREADS)
            .thread_name(|i| format!("nailpolish-io-{i}"))
            .build()
            .context("Failed to build I/O thread pool")?;

        Ok(Self {
            file: Arc::new(file),
            pool,
        })
    }

    fn make(&self) -> Result<PlainAccessor> {
        Ok(PlainAccessor {
            file: Arc::clone(&self.file),
        })
    }

    /// Given a number of reads to be requested, what is the optimal chunk size for
    /// parallelization?
    fn optimal_chunk_size(&self, num_reads: usize) -> usize {
        // optimally, split the number of reads into as many chunks as there are threads,
        // but don't make the chunks smaller than 1 read each.
        let n = PLAIN_THREADS.min(num_reads).max(1);
        num_reads.div_ceil(n)
    }
}

impl Source for PlainSource {
    fn fetch_byte_ranges(&self, ranges: &[ByteRange]) -> Result<Vec<Vec<u8>>> {
        let chunk_size = self.optimal_chunk_size(ranges.len());

        // create output buffer with slots for each read
        let mut output: Vec<Vec<u8>> = vec![Vec::new(); ranges.len()];

        // parallelize the read requests in chunks, each chunk serviced by a single thread
        // Rayon handles work-stealing and load balancing across threads
        self.pool.install(|| -> Result<()> {
            output
                .par_chunks_mut(chunk_size)
                .zip(ranges.par_chunks(chunk_size))
                .try_for_each(|(out_chunk, range_chunk)| -> Result<()> {
                    with_accessor(self, |accessor| {
                        for (out_slot, range) in out_chunk.iter_mut().zip(range_chunk) {
                            *out_slot = accessor.read(range.start, range.len())?;
                        }
                        Ok(())
                    })
                })
        })?;

        Ok(output)
    }
}

struct PlainAccessor {
    file: Arc<File>,
}

impl Accessor for PlainAccessor {
    #[cfg(unix)]
    fn read(&mut self, off: u64, len: u32) -> Result<Vec<u8>> {
        use std::os::unix::fs::FileExt;

        let mut buffer = vec![0; len as usize];
        self.file
            .read_exact_at(&mut buffer, off)
            .with_context(|| format!("Could not read {len} bytes at position {off}"))?;
        Ok(buffer)
    }
}
