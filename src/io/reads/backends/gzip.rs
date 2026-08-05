// Copyright 2026 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

//! Gzip random-access backend.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{Context, Result};
use indexed_deflate::GzDecoder;
use thiserror::Error;

use super::{Accessor, Source};

#[derive(Error, Debug)]
enum GzipSourceError {
    #[error("Gzip index file not found at {path}. Please run the indexing step first.")]
    IndexNotFound { path: String },
}

pub(crate) struct GzipSource {
    data_path: std::path::PathBuf,
    index_path: std::path::PathBuf,
    threads: usize,
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
        let num_threads = avail_cpus * 2;

        Ok(Self {
            data_path: path.to_path_buf(),
            index_path,
            threads: num_threads,
        })
    }
}

impl Source for GzipSource {
    fn make(&self) -> Result<Box<dyn Accessor>> {
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

        Ok(Box::new(GzipAccessor { decoder }))
    }

    fn threads(&self) -> usize {
        // CPU-bound: a seek costs decompressing up to ~1MB of discarded data, so one
        // inflate per core is the right budget. Must not run concurrently with the
        // consensus compute pool, since both are CPU-bound and would double-book cores.
        self.threads
    }

    fn optimal_chunk_size(&self, _num_reads: usize) -> usize {
        // set a fixed chunk size for now - in theory, an ideal chunk size should be big
        // enough to avoid repeat decompression, but small enough to benefit from
        // Rayon job-stealing
        128
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
