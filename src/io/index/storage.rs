//! `io::index::storage`
//!
//! This module is responsible for storing the index object on the filesystem.
//! It provides functionality to serialize, deserialize, and manage index data
//! for efficient storage and retrieval.

use crate::io::index::{ArchivedIndex, Index};

use anyhow::{Context, Result};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use memmap::Mmap;
use rkyv::{api::high::to_bytes_in, rancor};

pub struct IndexReader {
    file: FileIndexPath,
    mmap: Mmap,
}

impl IndexReader {
    pub fn new(file: FileIndexPath) -> Result<Self> {
        let file_obj = File::open(file.index())?;
        let mmap = unsafe { Mmap::map(&file_obj)? };

        Ok(Self { file, mmap })
    }

    pub fn load(&self) -> Result<&ArchivedIndex> {
        let idx: &ArchivedIndex = rkyv::access::<ArchivedIndex, rkyv::rancor::Error>(&self.mmap)?;
        Ok(idx)
    }
}

impl Index {
    /// Finalizes the writing process by flushing the writer, writing metadata,
    /// and copying the temporary file contents to the final output file.
    pub fn write(&self) -> Result<()> {
        let index_path = self.metadata.file_path.index();
        info!("Writing to {}...", index_path.display());

        let output = File::create_new(index_path)?;
        let buf_writer = BufWriter::new(output);
        let mut serializer = rkyv::ser::writer::IoWriter::new(buf_writer);

        to_bytes_in::<_, rancor::Error>(self, &mut serializer)?;
        Ok(())
    }
}

// TODO: remove when redundant
// pub fn write_index(index: &Index) -> Result<()> {
//     let index_path = index.metadata.file_path.index().clone();
//     info!("Writing to {}...", index_path.display());

//     let output = File::open(index_path)?;
//     let buf_writer = BufWriter::new(output);
//     let mut serializer = rkyv::ser::writer::IoWriter::new(buf_writer);

//     to_bytes_in::<_, rancor::Error>(index, &mut serializer)?;
//     Ok(())
// }

/// Represents the file paths for the input FASTQ file and the corresponding index file.
#[derive(Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct FileIndexPath {
    #[rkyv(with = rkyv::with::AsString)]
    path: PathBuf,

    #[rkyv(with = rkyv::with::AsString)]
    index: PathBuf,
}

impl FileIndexPath {
    /// Creates a new `FileIndexPath` instance from a given path.
    pub fn new(path: &Path) -> Self {
        let path = path.to_path_buf();
        let index = compute_index_path(&path);
        Self { path, index }
    }

    /// Returns a reference to the path of the input FASTQ file.
    pub fn fastq(&self) -> &PathBuf {
        &self.path
    }

    /// Returns a reference to the path of the index file.
    pub fn index(&self) -> &PathBuf {
        &self.index
    }

    pub fn check_indexed(&self) -> Result<()> {
        if self.index().exists() {
            Ok(())
        } else {
            anyhow::bail!(IndexReadErr::IndexDoesNotExist {
                path: std::path::absolute(self.index())?.display().to_string(),
                file: self.fastq().display().to_string(),
            })
        }
    }
}

/// Computes the path for the index file based on the input file path.
///
/// If the input file has the extension `.fastq`, the index file will have the
/// extension `.fastq.nailpolish.idx`.
fn compute_index_path(file: &Path) -> PathBuf {
    let index = file.to_path_buf();

    // if the original extension is ".fastq", the new one should be ".fastq.idx"
    let mut extension = file.extension().unwrap_or_default().to_os_string();
    extension.push(".nailpolish.idx");

    index.with_extension(extension)
}

#[derive(thiserror::Error, Debug)]
pub enum IndexReadErr {
    #[error(
        "
index file expected but not found at:
  {path}

suggestion: to generate an index, try `nailpolish index {file} --help`
suggestion: if you have recently moved {file}, move the .fastq.nailpolish.idx file as well"
    )]
    IndexDoesNotExist { path: String, file: String },
}
