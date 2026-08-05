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

/// A half-open byte range `[start, end)` within the uncompressed data stream.
#[derive(Copy, Clone, Debug)]
pub(crate) struct ByteRange {
    pub start: u64,
    pub end: u64,
}

impl ByteRange {
    /// Length of the range in bytes, as expected by [`Accessor::read`].
    pub(crate) fn len(&self) -> u32 {
        (self.end - self.start) as u32
    }
}

/// A source of reads which can be randomly accessed. Each backend owns its own
/// concurrency strategy — thread pool, accessor caching, and work splitting are all
/// private details, since the right approach differs between backends (plain files are
/// latency-bound, gzip is CPU-bound on inflate).
pub(crate) trait Source: Send + Sync {
    /// Fetch each requested byte range, returning the bytes in the same order as `ranges`.
    ///
    /// For best throughput `ranges` should be sorted by ascending `start`: backends read
    /// ranges near-sequentially within a worker, which avoids re-seeking and (for gzip)
    /// redundant decompression. Sorting is the **caller's** responsibility; correctness
    /// does not depend on it.
    fn fetch_byte_ranges(&self, ranges: &[ByteRange]) -> Result<Vec<Vec<u8>>>;
}
