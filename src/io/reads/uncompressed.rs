// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::{
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use anyhow::{Context, Result};
use thiserror::Error;

use super::GroupedReadsAccessor;
use crate::io::index::ReadLocation;

#[derive(Error, Debug)]
enum FileReadError {
    #[error("No reads were provided")]
    EmptyReadsProvided,
}

/// A reader that performs both sequential and random access reads from an uncompressed file
pub struct UncompressedFileReader {
    seq: SequentialReader,
    rnd: RandomAccessReader,
}

impl UncompressedFileReader {
    fn fetch_reads_random(&mut self, reads: &[ReadLocation]) -> Result<Vec<Vec<u8>>> {
        reads.iter().map(|read| self.rnd.fetch(read)).collect()
    }
}

impl GroupedReadsAccessor for UncompressedFileReader {
    fn new(file: &Path) -> Result<Self> {
        Ok(Self {
            seq: SequentialReader::new(file)?,
            rnd: RandomAccessReader::new(file)?,
        })
    }

    fn fetch_sequential_read(&mut self, read: &ReadLocation) -> Result<Vec<u8>> {
        self.seq.fetch(read)
    }

    fn fetch_reads(&mut self, reads: &[ReadLocation]) -> Result<Vec<Vec<u8>>> {
        // the first read (`head`) should be read using a sequential reader, while the
        // tail is not sequential and should be read using a random reader
        let (head, tail) = reads.split_at(1);
        let head = head.first().ok_or(FileReadError::EmptyReadsProvided)?;

        let mut reads = vec![self.seq.fetch(head)?];
        reads.extend(self.fetch_reads_random(tail)?);

        Ok(reads)
    }
}

pub trait SingleReadAccessor: Sized {
    fn new(file: &Path) -> Result<Self>;
    fn _fetch(&mut self, pos: u64, len: usize) -> Result<Vec<u8>>;

    fn fetch(&mut self, read: &ReadLocation) -> Result<Vec<u8>> {
        self._fetch(read.pos(), read.byte_len() as usize)
    }
}

/// A reader that performs random access reads from an uncompressed file
pub struct RandomAccessReader {
    file: std::fs::File,
}

impl SingleReadAccessor for RandomAccessReader {
    fn new(file: &Path) -> Result<Self> {
        let file = std::fs::File::open(file)?;
        Ok(Self { file })
    }

    fn _fetch(&mut self, pos: u64, num_bytes: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0; num_bytes];

        self.file
            .seek(SeekFrom::Start(pos))
            .with_context(|| format!("Unable to seek file at position {}", pos))?;

        self.file
            .read_exact(&mut buffer)
            .with_context(|| format!("Could not read {num_bytes} bytes at position {pos}"))?;

        debug!(
            "RandomAccessReader: read contents ({pos}, {num_bytes}):\n\n{}\n\n",
            std::str::from_utf8(&buffer).unwrap()
        );

        Ok(buffer)
    }
}

/// A reader that performs sequential reads from an uncompressed file using a buffered reader
pub struct SequentialReader {
    reader: BufReader<std::fs::File>,
    position: u64,
}

impl SingleReadAccessor for SequentialReader {
    fn new(file: &Path) -> Result<Self> {
        let file = std::fs::File::open(file)?;
        let reader = BufReader::with_capacity(*crate::env::READ_BUF_CAPACITY, file);

        Ok(Self {
            reader,
            position: 0,
        })
    }

    fn _fetch(&mut self, pos: u64, num_bytes: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0; num_bytes];

        let offset = (pos as i64) - (self.position as i64);
        if offset != 0 {
            debug!(
                "Offset is NOT zero:\n  {} -> {pos}, {num_bytes} bytes to read, offset: {offset}",
                self.position
            );
            self.reader.seek_relative(offset)?;
        }

        self.position = pos + num_bytes as u64;

        self.reader
            .read_exact(&mut buffer)
            .with_context(|| format!("Could not read {num_bytes} bytes at position {pos}"))?;

        debug!(
            "SequentialReader: read contents ({pos}, {num_bytes}):\n\n{}\n\n",
            std::str::from_utf8(&buffer).unwrap()
        );

        Ok(buffer)
    }
}
