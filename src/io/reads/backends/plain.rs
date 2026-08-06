// Copyright 2026 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use rayon::prelude::*;

use super::{Accessor, Chunk, Source};
use crate::utils::ByteRange;

const PLAIN_THREADS: usize = 64;
const PLAIN_MAX_SPAN: u64 = 64 * 1024;

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

/// Group `ranges` into contiguous spans, each no wider than `max_span`, so that a run of
/// nearby ranges can be serviced by a single read instead of one read per range.
/// The input ranges are assumed to be sorted by ascending `start`.
fn coalesce(ranges: &[ByteRange], max_span: u64) -> Vec<Chunk<'_>> {
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut chunk_start = 0;

    for (i, range) in ranges.iter().enumerate() {
        if let Some(open) = chunks.last_mut() {
            let fits =
                range.start >= open.start && range.end.saturating_sub(open.start) <= max_span;
            if fits {
                open.end = open.end.max(range.end);
                open.ranges = &ranges[chunk_start..=i];
                continue;
            }
        }

        // the open chunk (if any) cannot take this range: close it and start a new one. A
        // range wider than `max_span` on its own still gets a chunk to itself, since the span
        // is only checked against a chunk which already holds a range.
        chunk_start = i;
        chunks.push(Chunk {
            start: range.start,
            end: range.end,
            ranges: &ranges[i..=i],
        });
    }

    chunks
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

    fn fetch_chunk(&self, accessor: &mut dyn Accessor, chunk: &Chunk) -> Result<Vec<Vec<u8>>> {
        let buffer = accessor.read(chunk.start, chunk.len())?;

        chunk
            .ranges
            .iter()
            .map(|range| {
                let from = (range.start - chunk.start) as usize;
                Ok(buffer[from..from + range.len() as usize].to_vec())
            })
            .collect()
    }
}

impl Source for PlainSource {
    fn fetch_byte_ranges(&self, ranges: &[ByteRange]) -> Result<Vec<Vec<u8>>> {
        if ranges.is_empty() {
            return Ok(Vec::new());
        }

        // merge nearby ranges so that each chunk costs a single round-trip
        let chunks = coalesce(ranges, PLAIN_MAX_SPAN);

        let output: Vec<Vec<Vec<u8>>> = self.pool.install(|| {
            chunks
                .par_iter()
                .flat_map(|chunk| with_accessor(self, |accessor| self.fetch_chunk(accessor, chunk)))
                .collect()
        });

        Ok(output.into_iter().flatten().collect())
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
