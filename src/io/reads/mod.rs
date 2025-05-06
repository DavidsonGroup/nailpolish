pub mod record;
pub mod uncompressed;

use super::index::{ArchivedReadLocation, ReadLocationTrait};
use crate::io::index::ReadLocation;

use anyhow::Result;
use bio::io::fastq::{self, FastqRead};
use std::io::Cursor;

const BUF_CAPACITY: usize = 1024usize.pow(2);

/// An in-memory sequence record which performs an allocation. Independent of read status.
pub type InMemorySequenceRecord = bio::io::fastq::Record;

pub trait RecordLoader: Sized {
    fn from_u8(buf: Vec<u8>) -> Result<Self>;
}

impl RecordLoader for bio::io::fastq::Record {
    fn from_u8(buf: Vec<u8>) -> Result<Self> {
        let cursor = Cursor::new(buf);
        let mut reader = fastq::Reader::new(cursor);

        let mut record = fastq::Record::new();
        reader.read(&mut record)?;

        Ok(record)
    }
}

pub trait GroupedReadsAccessor {
    fn new(file: &std::path::Path) -> Result<Self>
    where
        Self: Sized;

    fn fetch_reads(&mut self, reads: &[ReadLocation]) -> Result<Vec<InMemorySequenceRecord>>;

    fn fetch_reads_archived(
        &mut self,
        reads: &[ArchivedReadLocation],
    ) -> Result<Vec<InMemorySequenceRecord>> {
        let reads = reads
            .iter()
            .map(|v| ReadLocation {
                _pos: v.pos(),
                _byte_len: v.byte_len(),
            })
            .collect::<Vec<_>>();
        self.fetch_reads(&reads)
    }
}
