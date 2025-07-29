// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use anyhow::{Context, Result};
use indexed_deflate::GzDecoder;
use thiserror::Error;

use super::GroupedReadsAccessor;
use crate::io::index::ReadLocation;

#[derive(Error, Debug)]
enum GzipReadError {
    #[error("No reads were provided")]
    EmptyReadsProvided,
    
    #[error("Gzip index file not found at {path}. Please run the indexing step first.")]
    IndexNotFound { path: String },
}

/// A reader that performs both sequential and random access reads from a gzip-compressed file
/// using indexed_deflate for efficient random access
pub struct GzippedFileReader {
    seq: SequentialGzReader,
    rnd: RandomAccessGzReader,
}

impl GzippedFileReader {
    fn fetch_reads_random(&mut self, reads: &[ReadLocation]) -> Result<Vec<Vec<u8>>> {
        reads.iter().map(|read| self.rnd.fetch(read)).collect()
    }
}

impl GroupedReadsAccessor for GzippedFileReader {
    fn new(file: &Path) -> Result<Self> {
        Ok(Self {
            seq: SequentialGzReader::new(file)?,
            rnd: RandomAccessGzReader::new(file)?,
        })
    }

    fn fetch_sequential_read(&mut self, read: &ReadLocation) -> Result<Vec<u8>> {
        self.seq.fetch(read)
    }

    fn fetch_reads(&mut self, reads: &[ReadLocation]) -> Result<Vec<Vec<u8>>> {
        // the first read (`head`) should be read using a sequential reader, while the
        // tail is not sequential and should be read using a random reader
        let (head, tail) = reads.split_at(1);
        let head = head.first().ok_or(GzipReadError::EmptyReadsProvided)?;

        let mut reads = vec![self.seq.fetch(head)?];
        reads.extend(self.fetch_reads_random(tail)?);

        Ok(reads)
    }
}

pub trait SingleGzReadAccessor: Sized {
    fn new(file: &Path) -> Result<Self>;
    fn _fetch(&mut self, pos: u64, len: usize) -> Result<Vec<u8>>;

    fn fetch(&mut self, read: &ReadLocation) -> Result<Vec<u8>> {
        self._fetch(read.pos(), read.byte_len() as usize)
    }
}

/// A reader that performs random access reads from a gzip-compressed file
pub struct RandomAccessGzReader {
    decoder: GzDecoder<File, File>,
}

impl SingleGzReadAccessor for RandomAccessGzReader {
    fn new(file: &Path) -> Result<Self> {
        let index_path = crate::utils::get_gzip_index_path(file);
        
        if !index_path.exists() {
            return Err(GzipReadError::IndexNotFound {
                path: index_path.display().to_string(),
            }.into());
        }

        let data_file = File::open(file)?;
        let index_file = File::open(&index_path)?;
        
        let decoder = GzDecoder::new(data_file, index_file)
            .context("Failed to create GzDecoder")?;
        
        Ok(Self { decoder })
    }

    fn _fetch(&mut self, pos: u64, num_bytes: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0; num_bytes];

        self.decoder
            .seek(SeekFrom::Start(pos))
            .with_context(|| format!("Unable to seek gzip file at position {}", pos))?;

        self.decoder
            .read_exact(&mut buffer)
            .with_context(|| format!("Could not read {num_bytes} bytes at position {pos}"))?;

        debug!(
            "RandomAccessGzReader: read contents ({pos}, {num_bytes}):\n\n{}\n\n",
            std::str::from_utf8(&buffer).unwrap()
        );

        Ok(buffer)
    }
}

/// A reader that performs sequential reads from a gzip-compressed file
pub struct SequentialGzReader {
    decoder: GzDecoder<File, File>,
    position: u64,
}

impl SingleGzReadAccessor for SequentialGzReader {
    fn new(file: &Path) -> Result<Self> {
        let index_path = crate::utils::get_gzip_index_path(file);
        
        if !index_path.exists() {
            return Err(GzipReadError::IndexNotFound {
                path: index_path.display().to_string(),
            }.into());
        }

        let data_file = File::open(file)?;
        let index_file = File::open(&index_path)?;
        
        let decoder = GzDecoder::new(data_file, index_file)
            .context("Failed to create GzDecoder")?;

        Ok(Self {
            decoder,
            position: 0,
        })
    }

    fn _fetch(&mut self, pos: u64, num_bytes: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0; num_bytes];

        let offset = (pos as i64) - (self.position as i64);
        if offset != 0 {
            debug!(
                "Gzip offset is NOT zero:\n  {} -> {pos}, {num_bytes} bytes to read, offset: {offset}",
                self.position
            );
            self.decoder.seek(SeekFrom::Start(pos))?;
        }

        self.position = pos + num_bytes as u64;

        self.decoder
            .read_exact(&mut buffer)
            .with_context(|| format!("Could not read {num_bytes} bytes at position {pos}"))?;

        debug!(
            "SequentialGzReader: read contents ({pos}, {num_bytes}):\n\n{}\n\n",
            std::str::from_utf8(&buffer).unwrap()
        );

        Ok(buffer)
    }
}