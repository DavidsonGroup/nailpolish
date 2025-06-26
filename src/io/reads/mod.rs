pub mod record;
pub mod uncompressed;

use crate::io::index::{DuplicateGroup, DuplicateGroupLocation, DuplicateGroupType, ReadLocation};
pub use record::QualityCompute;

use anyhow::Result;

pub trait GroupedReadsAccessor {
    fn new(file: &std::path::Path) -> Result<Self>
    where
        Self: Sized;

    fn fetch_reads(&mut self, reads: &[ReadLocation]) -> Result<Vec<Vec<u8>>>;
    fn fetch_reads_random(&mut self, reads: &[ReadLocation]) -> Result<Vec<Vec<u8>>>;

    fn fetch_group(&mut self, group: &DuplicateGroupLocation) -> Result<Vec<Vec<u8>>> {
        self.fetch_reads(&group.reads)
    }
}
