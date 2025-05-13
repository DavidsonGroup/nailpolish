use super::{GroupedReadsAccessor, InMemorySequenceRecord, RecordLoader, BUF_CAPACITY};
use crate::io::index::{ReadLocation, ReadLocationTrait};

use anyhow::{Context, Result};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use thiserror::Error;

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

impl GroupedReadsAccessor for UncompressedFileReader {
    fn new(file: &Path) -> Result<Self> {
        Ok(Self {
            seq: SequentialReader::new(file)?,
            rnd: RandomAccessReader::new(file)?,
        })
    }

    fn fetch_reads(&mut self, reads: &[ReadLocation]) -> Result<Vec<InMemorySequenceRecord>> {
        let len = reads.len();

        // the first read (`head`) should be read using a sequential reader, while the
        // tail is not sequential and should be read using a random reader
        let (head, tail) = reads.split_at(1);
        let head = head.first().ok_or(FileReadError::EmptyReadsProvided)?;

        let first_read = self.seq.fetch(head)?;

        // create the remaining reads
        let mut result = Vec::with_capacity(len);
        result.push(first_read);
        for rem in tail {
            let read = self.rnd.fetch(rem)?;
            result.push(read);
        }

        Ok(result)
    }
}

pub trait SingleReadAccessor: Sized {
    fn new(file: &Path) -> Result<Self>;
    fn _fetch(&mut self, pos: u64, num_bytes: u64) -> Result<InMemorySequenceRecord>;

    fn fetch(&mut self, read: &ReadLocation) -> Result<InMemorySequenceRecord> {
        self._fetch(read.pos() as u64, read.byte_len() as u64)
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

    fn _fetch(&mut self, pos: u64, num_bytes: u64) -> Result<InMemorySequenceRecord> {
        self.file
            .seek(SeekFrom::Start(pos))
            .with_context(|| format!("Unable to seek file at position {}", pos))?;

        let mut buffer = vec![0; num_bytes as usize];
        self.file
            .read_exact(&mut buffer)
            .with_context(|| format!("Could not read {num_bytes} bytes at position {pos}"))?;

        InMemorySequenceRecord::from_u8(buffer)
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
        let reader = BufReader::with_capacity(BUF_CAPACITY, file);

        Ok(Self {
            reader,
            position: 0,
        })
    }

    fn _fetch(&mut self, pos: u64, num_bytes: u64) -> Result<InMemorySequenceRecord> {
        let offset = (pos as i64) - (self.position as i64);
        if offset != 0 {
            debug!(
                "Offset is NOT zero:\n  {} -> {pos}, {num_bytes} bytes to read, offset: {offset}",
                self.position
            );
            self.reader.seek_relative(offset)?;
        }

        self.position = pos + num_bytes;

        let mut buffer = vec![0; num_bytes as usize];
        self.reader
            .read_exact(&mut buffer)
            .with_context(|| format!("Could not read {num_bytes} bytes at position {pos}"))?;

        InMemorySequenceRecord::from_u8(buffer)
    }
}
