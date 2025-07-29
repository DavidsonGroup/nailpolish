// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::fs::File;
use std::io::{BufRead, BufReader, Result, Seek, SeekFrom};
use std::path::Path;

use anyhow::Context;
use indexed_deflate::{AccessPointSpan, GzIndexBuilder};

/// A reader that handles both uncompressed and gzip-compressed FASTQ files
/// during sequential indexing operations. For gzip files, it builds an index
/// progressively as the file is read.
pub enum SequentialIndexedReader {
    Uncompressed(BufReader<File>),
    Gzipped(BufReader<GzIndexBuilder<File, File>>),
}

impl SequentialIndexedReader {
    /// Creates a new reader for an uncompressed FASTQ file
    pub fn new_uncompressed(file: File) -> Self {
        Self::Uncompressed(BufReader::new(file))
    }

    /// Creates a new reader for a gzip-compressed FASTQ file
    /// The gzip index will be built progressively as the file is read
    pub fn new_gzipped(gz_file: File, index_file: File) -> anyhow::Result<Self> {
        let builder = GzIndexBuilder::new(gz_file, index_file, AccessPointSpan::default())
            .context("Failed to create GzIndexBuilder")?;
        Ok(Self::Gzipped(BufReader::new(builder)))
    }

    /// Creates a reader from a file path, automatically detecting compression
    pub fn from_path(file_path: &Path) -> anyhow::Result<Self> {
        let file = File::open(file_path)?;
        
        if crate::utils::is_gzip_file(file_path) {
            // Create gzip index file path
            let gz_index_path = file_path.with_extension("gzi2");
            
            // Create or open the gzip index file
            let index_file = File::options()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .open(&gz_index_path)?;
            
            Self::new_gzipped(file, index_file)
        } else {
            Ok(Self::new_uncompressed(file))
        }
    }

    /// Finalizes the gzip index if this is a gzipped reader
    /// This should be called after all reading is complete
    pub fn finish_gzip_index(self) -> anyhow::Result<()> {
        match self {
            Self::Gzipped(buf_reader) => {
                // Extract the inner GzIndexBuilder from BufReader
                let builder = buf_reader.into_inner();
                builder.finish().context("Failed to finalize gzip index")?;
                Ok(())
            }
            Self::Uncompressed(_) => Ok(()),
        }
    }
}

impl BufRead for SequentialIndexedReader {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        match self {
            Self::Uncompressed(reader) => reader.fill_buf(),
            Self::Gzipped(reader) => reader.fill_buf(),
        }
    }

    fn consume(&mut self, amt: usize) {
        match self {
            Self::Uncompressed(reader) => reader.consume(amt),
            Self::Gzipped(reader) => reader.consume(amt),
        }
    }
}

impl std::io::Read for SequentialIndexedReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            Self::Uncompressed(reader) => reader.read(buf),
            Self::Gzipped(reader) => reader.read(buf),
        }
    }
}

impl Seek for SequentialIndexedReader {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        match self {
            Self::Uncompressed(reader) => reader.seek(pos),
            Self::Gzipped(reader) => reader.seek(pos),
        }
    }

    fn stream_position(&mut self) -> Result<u64> {
        match self {
            Self::Uncompressed(reader) => reader.stream_position(),
            Self::Gzipped(reader) => reader.stream_position(),
        }
    }
}