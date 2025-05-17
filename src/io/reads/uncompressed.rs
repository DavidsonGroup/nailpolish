use super::GroupedReadsAccessor;
use crate::io::index::{ReadLocation, ReadLocationTrait};

use anyhow::{bail, Context, Result};
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

    fn fetch_reads(&mut self, reads: &[ReadLocation]) -> Result<Vec<u8>> {
        let buffer_size = reads.iter().map(|v| v._byte_len as usize).sum();
        let mut buffer = vec![0; buffer_size];

        // the first read (`head`) should be read using a sequential reader, while the
        // tail is not sequential and should be read using a random reader
        let (head, tail) = reads.split_at(1);
        let head = head.first().ok_or(FileReadError::EmptyReadsProvided)?;

        // fetch first read using sequential reader
        self.seq
            .fetch(head, &mut buffer[0..head._byte_len as usize])
            .with_context(|| format!("Could not fetch read at {head:?}"))?;

        // fetch remaining reads using random reader
        self._fetch_reads_random(tail, &mut buffer[head._byte_len as usize..])?;

        Ok(buffer)
    }

    fn _fetch_reads_random(&mut self, reads: &[ReadLocation], buffer: &mut [u8]) -> Result<()> {
        let mut start = 0;
        for read in reads.iter() {
            let end = start + read._byte_len as usize;

            if end > buffer.len() {
                bail!(
                    "Buffer size {} is too small for read with end {end} at {read:?}",
                    buffer.len()
                );
            }

            // fetch the read into the buffer
            self.rnd
                .fetch(read, &mut buffer[start..end])
                .with_context(|| format!("Could not fetch read at {read:?}"))?;

            start = end;
        }

        Ok(())
    }

    fn fetch_reads_random(&mut self, reads: &[ReadLocation]) -> Result<Vec<u8>> {
        let buffer_size = reads.iter().map(|v| v._byte_len as usize).sum();
        let mut buffer = vec![0; buffer_size];

        self._fetch_reads_random(reads, &mut buffer)?;
        Ok(buffer)
    }
}

pub trait SingleReadAccessor: Sized {
    fn new(file: &Path) -> Result<Self>;
    fn _fetch(&mut self, pos: u64, buffer: &mut [u8]) -> Result<()>;

    fn fetch(&mut self, read: &ReadLocation, buffer: &mut [u8]) -> Result<()> {
        self._fetch(read.pos() as u64, buffer)
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

    fn _fetch(&mut self, pos: u64, buffer: &mut [u8]) -> Result<()> {
        let num_bytes = buffer.len();

        self.file
            .seek(SeekFrom::Start(pos))
            .with_context(|| format!("Unable to seek file at position {}", pos))?;

        self.file
            .read_exact(buffer)
            .with_context(|| format!("Could not read {num_bytes} bytes at position {pos}"))?;

        debug!(
            "RandomAccessReader: read contents ({pos}, {num_bytes}):\n\n{}\n\n",
            std::str::from_utf8(&buffer).unwrap()
        );

        Ok(())
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

    fn _fetch(&mut self, pos: u64, buffer: &mut [u8]) -> Result<()> {
        let num_bytes = buffer.len();

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
            .read_exact(buffer)
            .with_context(|| format!("Could not read {num_bytes} bytes at position {pos}"))?;

        debug!(
            "SequentialReader: read contents ({pos}, {num_bytes}):\n\n{}\n\n",
            std::str::from_utf8(&buffer).unwrap()
        );

        Ok(())
    }
}
