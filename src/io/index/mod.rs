// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

pub mod construct;
pub mod filter;
pub mod metadata;
pub mod record_identifier;
pub mod storage;

use std::time::SystemTimeError;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use needletail::parser::SequenceRecord;
use rkyv::{
    option::ArchivedOption, string::ArchivedString, vec::ArchivedVec, Archive, Deserialize,
    Serialize,
};
use smallvec::{smallvec, SmallVec};

use crate::io::index::record_identifier::ArchivedRecordIdentifier;
use crate::io::reads::gzipped::GzippedFileReader;
use crate::io::reads::uncompressed::UncompressedFileReader;
use crate::io::reads::GroupedReadsAccessor;
use crate::utils::deserialize_standard;

use metadata::IndexMetadata;
pub use record_identifier::RecordIdentifier;
pub use storage::{FileIndexPath, IndexReader};

// An individual indexed read, with its byte length in file
// and read status
#[derive(Archive, Serialize, Deserialize, Clone, Debug)]
#[rkyv(derive(Debug))]
pub struct ReadLocation {
    pub _pos: u64,
    pub _byte_len: u32,
    pub seq_len: u32,
    pub qual: f32,
}

impl ReadLocation {
    pub fn pos(&self) -> u64 {
        self._pos
    }

    pub fn byte_len(&self) -> u32 {
        self._byte_len
    }
}

/// Generated on demand when iterating through the index.
pub struct DuplicateGroupLocation {
    pub key: RecordIdentifier,
    pub reads: SmallVec<[ReadLocation; 1]>,
    pub id: usize,
}

impl DuplicateGroupLocation {
    pub fn is_simplex(&self) -> bool {
        self.reads.len() == 1
    }
}

pub struct DuplicateGroup {
    pub key: RecordIdentifier,
    pub reads: Vec<Vec<u8>>,
    pub id: usize,
    pub group_type: DuplicateGroupType,
}

#[derive(PartialEq, Debug)]
pub enum DuplicateGroupType {
    Valid,
    Filtered,
}

/// Represents an index structure for managing duplicate groups and their associated metadata.
#[derive(Archive, Deserialize, Serialize)]
pub struct Index {
    /// A map of duplicate group keys to their associated read locations.
    groups: IndexMap<RecordIdentifier, SmallVec<[ReadLocation; 1]>>,

    /// Metadata associated with the index.
    metadata: IndexMetadata,

    /// The start time of the indexation process.
    #[rkyv(with = rkyv::with::AsUnixTime)]
    _start: std::time::SystemTime,

    /// The current path of the index and file. This should be set to the invocation path upon
    /// index loading, unlike metadata.file_path
    path: FileIndexPath,

    /// Comments/tags to emit
    captures: Vec<Option<String>>,
}

impl Index {
    /// Creates a new `Index` instance with the given file path.
    pub fn new(file: FileIndexPath) -> Self {
        let mut index = Self {
            groups: IndexMap::new(),
            metadata: IndexMetadata::default(),
            _start: std::time::SystemTime::now(),
            path: file.clone(),
            captures: vec![],
        };
        index.metadata.add_general_metadata(file);
        index
    }

    /// Add regular expression capture groups
    pub fn add_capture_groups(&mut self, re: &regex::Regex) {
        let names = re
            .capture_names() // get capture names
            .skip(1); // we skip over the first result, since this is the entire capture group

        self.captures = names
            .map(|v| v.map(str::to_string)) // convert all Some(&str) to Some(String)
            .collect()
    }

    /// Retrieves a duplicate group by its ID.
    pub fn get_by_id(&self, index: usize) -> Option<DuplicateGroupLocation> {
        self.groups
            .get_index(index)
            .map(|(key, reads)| DuplicateGroupLocation {
                key: key.clone(),
                reads: reads.clone(),
                id: index,
            })
    }

    /// Retrieves a duplicate group by its record identifier.
    pub fn get_by_record_key(&self, key: RecordIdentifier) -> Option<DuplicateGroupLocation> {
        self.get_by_key(&key)
    }

    /// Adds a read to the index under the specified record identifier
    pub fn add_read(&mut self, key: RecordIdentifier, read: ReadLocation, seq: SequenceRecord) {
        self.metadata.add_read_metadata(seq);
        self.add_read_entry(key, read)
    }

    /// Returns a reference to the metadata associated with the index.
    pub fn metadata(&self) -> &IndexMetadata {
        &self.metadata
    }

    /// Marks the indexation process as complete and updates metadata with elapsed time and size.
    pub fn mark_indexation_complete(&mut self, gb: f64) -> Result<(), SystemTimeError> {
        self.metadata.elapsed = self._start.elapsed()?.as_secs_f64();
        self.metadata.gb = gb;
        Ok(())
    }

    /// Retrieves a duplicate group by its key.
    fn get_by_key(&self, key: &RecordIdentifier) -> Option<DuplicateGroupLocation> {
        self.groups
            .get_full(key)
            .map(|(index, key, reads)| DuplicateGroupLocation {
                key: key.clone(),
                reads: reads.clone(),
                id: index,
            })
    }

    /// Adds a read entry to the index under the specified duplicate group key.
    fn add_read_entry(&mut self, key: RecordIdentifier, read: ReadLocation) {
        self.groups
            .entry(key)
            .and_modify(|e| e.push(read.clone()))
            .or_insert_with(|| smallvec![read]);
    }
}

impl ArchivedIndex {
    /// Returns an iterator over the duplicate groups in the index.
    pub fn groups(&self) -> impl Iterator<Item = DuplicateGroupLocation> + use<'_> {
        self.groups
            .iter()
            .enumerate()
            .map(|(index, (key, value))| DuplicateGroupLocation {
                key: deserialize_standard(key),
                reads: deserialize_standard(value),
                id: index,
            })
    }

    pub fn get_hashmap(
        &self,
    ) -> &rkyv::collections::swiss_table::ArchivedIndexMap<
        ArchivedRecordIdentifier,
        ArchivedVec<ArchivedReadLocation>,
    > {
        &self.groups
    }

    /// Get an read accessor that can perform filesystem operations and read the original reads
    pub fn get_read_accessor(
        &self,
        index_path: &FileIndexPath,
    ) -> Result<Box<dyn GroupedReadsAccessor>> {
        let fastq_path = index_path.fastq();
        
        if crate::utils::is_gzip_file(fastq_path) {
            let reader = GzippedFileReader::new(fastq_path)
                .with_context(|| format!("Error reading gzipped read file {}", fastq_path.display()))?;
            Ok(Box::new(reader))
        } else {
            let reader = UncompressedFileReader::new(fastq_path)
                .with_context(|| format!("Error reading read file {}", fastq_path.display()))?;
            Ok(Box::new(reader))
        }
    }

    /// Retrieves a duplicate group by its ID.
    pub fn get_by_id(&self, index: usize) -> Option<DuplicateGroupLocation> {
        self.groups
            .get_index(index)
            .map(|(key, reads)| DuplicateGroupLocation {
                key: deserialize_standard(key),
                reads: deserialize_standard(reads),
                id: index,
            })
    }

    /// Provides access to the metadata of the index.
    pub fn metadata(&self) -> IndexMetadata {
        deserialize_standard(&self.metadata)
    }

    pub fn captures(&self) -> &ArchivedVec<ArchivedOption<ArchivedString>> {
        &self.captures
    }
}
