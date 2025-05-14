pub mod record;
pub mod uncompressed;

use super::index::{ArchivedReadLocation, ReadLocationTrait};
use crate::io::index::ReadLocation;
pub use record::QualityCompute;

use anyhow::Result;
use bio::io::fastq::{self, FastqRead};
use std::io::Cursor;

const BUF_CAPACITY: usize = 2 * 1024usize.pow(2);

/// Read sequence record handling and loading.
///
/// Provides traits and types for working with FASTQ sequence records in memory.
pub type InMemorySequenceRecord = bio::io::fastq::Record;

/// Loads sequence records from raw bytes.
pub trait RecordLoader: Sized {
    /// Creates a new record from raw bytes.
    fn from_u8(buf: Vec<u8>) -> Result<Self>;
}

/// FASTQ record loader implementation.
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
    fn fetch_reads_random(&mut self, reads: &[ReadLocation])
        -> Result<Vec<InMemorySequenceRecord>>;

    fn fetch_reads_archived(
        &mut self,
        reads: &[ArchivedReadLocation],
    ) -> Result<Vec<InMemorySequenceRecord>> {
        let reads = reads.iter().map(ReadLocation::from).collect::<Vec<_>>();
        self.fetch_reads(&reads)
    }

    fn fetch_reads_random_archived(
        &mut self,
        reads: &[ArchivedReadLocation],
    ) -> Result<Vec<InMemorySequenceRecord>> {
        let reads = reads.iter().map(ReadLocation::from).collect::<Vec<_>>();
        self.fetch_reads_random(&reads)
    }
}

impl From<&ArchivedReadLocation> for ReadLocation {
    fn from(v: &ArchivedReadLocation) -> Self {
        ReadLocation {
            _pos: v.pos(),
            _byte_len: v.byte_len(),
            seq_len: v.seq_len.try_into().unwrap(),
            qual: v.qual.into(),
        }
    }
}
