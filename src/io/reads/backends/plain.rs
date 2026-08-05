// Copyright 2026 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

//! Plain (uncompressed) random-access backend.

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};

use super::{Accessor, Source};

/// Plain files are latency-bound: each read is a network round-trip of a few hundred
/// microseconds, and threads waiting on one are parked, not running. A high thread
/// count gives queue depth against that latency.
const PLAIN_THREADS: usize = 128;

pub(crate) struct PlainSource {
    file: Arc<File>,
}

impl PlainSource {
    pub(crate) fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)
            .with_context(|| format!("Error opening read file {}", path.display()))?;
        Ok(Self {
            file: Arc::new(file),
        })
    }
}

impl Source for PlainSource {
    fn make(&self) -> Result<Box<dyn Accessor>> {
        Ok(Box::new(PlainAccessor {
            file: Arc::clone(&self.file),
        }))
    }

    fn threads(&self) -> usize {
        PLAIN_THREADS
    }

    fn optimal_chunk_size(&self, num_reads: usize) -> usize {
        // optimally, split the number of reads into as many chunks as there are threads,
        // but don't make the chunks smaller than 1 read each.
        let n = self.threads().min(num_reads).max(1);
        num_reads.div_ceil(n)
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
