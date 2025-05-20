pub mod record;
pub mod uncompressed;

use super::index::{ArchivedReadLocation, ReadLocationTrait};
use crate::io::index::ReadLocation;
pub use record::QualityCompute;

use anyhow::Result;

pub trait GroupedReadsAccessor {
    fn new(file: &std::path::Path) -> Result<Self>
    where
        Self: Sized;

    fn fetch_reads(&mut self, reads: &[ReadLocation]) -> Result<Vec<u8>>;
    fn fetch_reads_random(&mut self, reads: &[ReadLocation]) -> Result<Vec<u8>>;
    fn _fetch_reads_random(&mut self, reads: &[ReadLocation], buffer: &mut [u8]) -> Result<()>;

    fn fetch_reads_archived(&mut self, reads: &[ArchivedReadLocation]) -> Result<Vec<u8>> {
        let reads = reads.iter().map(ReadLocation::from).collect::<Vec<_>>();
        self.fetch_reads(&reads)
    }

    fn fetch_reads_random_archived(&mut self, reads: &[ArchivedReadLocation]) -> Result<Vec<u8>> {
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
