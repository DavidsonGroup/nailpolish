// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

pub mod record;
pub mod sequential_indexed_reader;
pub mod uncompressed;

use anyhow::Result;

use crate::io::index::{DuplicateGroupLocation, ReadLocation};

pub use record::QualityCompute;
pub use sequential_indexed_reader::SequentialIndexedReader;

pub trait GroupedReadsAccessor {
    fn new(file: &std::path::Path) -> Result<Self>
    where
        Self: Sized;

    fn fetch_sequential_read(&mut self, read: &ReadLocation) -> Result<Vec<u8>>;

    fn fetch_reads(&mut self, reads: &[ReadLocation]) -> Result<Vec<Vec<u8>>>;

    fn fetch_group(&mut self, group: &DuplicateGroupLocation) -> Result<Vec<Vec<u8>>> {
        self.fetch_reads(&group.reads)
    }
}
