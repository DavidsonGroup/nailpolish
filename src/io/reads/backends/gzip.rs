// Copyright 2026 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

//! Gzip random-access backend.

use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{Context, Result};
use indexed_deflate::GzDecoder;
use rayon::prelude::*;
use thiserror::Error;

use super::{Accessor, ByteRange, Source};

/// Fixed chunk size for now - in theory, an ideal chunk size should be big enough to avoid
/// repeat decompression, but small enough to benefit from Rayon job-stealing
const GZIP_CHUNK_SIZE: usize = 128;

#[derive(Error, Debug)]
enum GzipSourceError {
    #[error("Gzip index file not found at {path}. Please run the indexing step first.")]
    IndexNotFound { path: String },
}

thread_local! {
    static ACCESSOR: RefCell<Option<GzipAccessor>> = const { RefCell::new(None) };
}

/// Access the underlying accessor for the given source, creating it if necessary. This is used to
/// ensure that the accessor is only created once per thread, and that it is reused for all
/// subsequent read requests.
fn with_accessor<T>(src: &GzipSource, f: impl FnOnce(&mut GzipAccessor) -> T) -> T {
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

pub(crate) struct GzipSource {
    data_path: std::path::PathBuf,
    index_path: std::path::PathBuf,
    pool: rayon::ThreadPool,
}

impl GzipSource {
    pub(crate) fn new(
        path: &Path,
        index_path: std::path::PathBuf,
        avail_cpus: usize,
    ) -> Result<Self> {
        if !index_path.exists() {
            return Err(GzipSourceError::IndexNotFound {
                path: index_path.display().to_string(),
            }
            .into());
        }

        // this is a hard coded value for now - does it need to be configurable?
        // The gzip backend is CPU-bound, so we want to limit the number of threads to avoid;
        // at the same time, I/O is a source of considerable latency, so we want enough threads
        // to keep the CPU busy while waiting for I/O.
        //
        // CPU-bound: a seek costs decompressing up to ~1MB of discarded data, so one
        // inflate per core is the right budget. Must not run concurrently with the
        // consensus compute pool, since both are CPU-bound and would double-book cores.
        let num_threads = avail_cpus * 2;

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .thread_name(|i| format!("nailpolish-io-{i}"))
            .build()
            .context("Failed to build I/O thread pool")?;

        Ok(Self {
            data_path: path.to_path_buf(),
            index_path,
            pool,
        })
    }

    fn make(&self) -> Result<GzipAccessor> {
        let data_file = File::open(&self.data_path)
            .with_context(|| format!("Error opening read file {}", self.data_path.display()))?;
        let index_file = File::open(&self.index_path).with_context(|| {
            format!(
                "Error opening gzip index file {}",
                self.index_path.display()
            )
        })?;

        let decoder =
            GzDecoder::new(data_file, index_file).context("Failed to create GzDecoder")?;

        Ok(GzipAccessor { decoder })
    }
}

impl Source for GzipSource {
    fn fetch_byte_ranges(&self, ranges: &[ByteRange]) -> Result<Vec<Vec<u8>>> {
        let chunk_size = GZIP_CHUNK_SIZE;

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

struct GzipAccessor {
    decoder: GzDecoder<File, File>,
}

impl Accessor for GzipAccessor {
    fn read(&mut self, off: u64, len: u32) -> Result<Vec<u8>> {
        let mut buffer = vec![0; len as usize];

        self.decoder
            .seek(SeekFrom::Start(off))
            .with_context(|| format!("Unable to seek gzip file at position {off}"))?;

        self.decoder
            .read_exact(&mut buffer)
            .with_context(|| format!("Could not read {len} bytes at position {off}"))?;

        Ok(buffer)
    }
}
