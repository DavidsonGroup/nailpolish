// Copyright 2026 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use anyhow::Result;

pub(crate) mod gzip;
pub(crate) mod plain;

pub(crate) trait Accessor: Send {
    fn read(&mut self, off: u64, len: u32) -> Result<Vec<u8>>;
}

/// Factory for `Accessor`
pub(crate) trait Source: Send + Sync {
    fn make(&self) -> Result<Box<dyn Accessor>>;

    /// How many threads should be used for this backend? This is used to tune the
    /// thread pool size for the random access reader and will depend on the primary
    /// performance restriction (compute vs I/O)
    fn threads(&self) -> usize;

    /// Given a number of reads to be requested, what is the optimal chunk size for
    /// parallelization? This is used to tune the chunk size for the random access reader
    /// and will depend on the primary performance restriction (compute vs I/O)
    fn optimal_chunk_size(&self, num_reads: usize) -> usize;
}
