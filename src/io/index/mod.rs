pub mod construct;
pub mod filter;
pub mod metadata;
pub mod record_identifier;
pub mod storage;

use metadata::IndexMetadata;

pub use crate::io::index::storage::{FileIndexPath, IndexReader};
use crate::io::reads::uncompressed::UncompressedFileReader;
use crate::io::reads::GroupedReadsAccessor;
pub use record_identifier::RecordIdentifier;

use anyhow::{Context, Result};
use std::time::SystemTimeError;

use indexmap::IndexMap;
use needletail::parser::SequenceRecord;
use rkyv::{rancor, vec::ArchivedVec, Archive, Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

// An individual indexed read, with its byte length in file
// and read status
#[derive(Archive, Serialize, Deserialize, Clone, Debug)]
pub struct ReadLocation {
    pub _pos: u64,
    pub _byte_len: u32,
    pub seq_len: u32,
    pub qual: f32,
}

pub trait ReadLocationTrait {
    fn pos(&self) -> u64;
    fn byte_len(&self) -> u32;
}

macro_rules! impl_read_loc_trait {
    ($($tys:ty), *) => {
        $(
            impl ReadLocationTrait for $tys {
                fn pos(&self) -> u64 {
                    u64::try_from(self._pos).unwrap()
                }

                fn byte_len(&self) -> u32 {
                    u32::try_from(self._byte_len).unwrap()
                }
            }
        )*
    }
}

impl_read_loc_trait!(ReadLocation, ArchivedReadLocation);

/// Generated on demand when iterating through the index.
pub struct DuplicateGroup<'a> {
    pub key: &'a DuplicateGroupKey,
    pub reads: &'a SmallVec<[ReadLocation; 1]>,
    pub index: usize,
}

pub struct ArchivedDuplicateGroup<'a> {
    pub key: &'a ArchivedDuplicateGroupKey,
    pub reads: &'a ArchivedVec<ArchivedReadLocation>,
    pub id: usize,
}

/// Represents the index of a duplicate group.
///
/// - `Normal`: A group with a valid `RecordIdentifier`.
/// - `Ignored` and `Filtered`: A group that is ignored, represented by a unique value. This can be
///   the read's byte position. It is important that this is guaranteed unique, as this prevents
///   collisions and overwrites from occurring.
#[derive(Archive, Serialize, Hash, Eq, PartialEq)]
pub enum DuplicateGroupKey {
    Normal(RecordIdentifier),
    Invalid(u64),
    Filtered(u64),
}

impl DuplicateGroupKey {
    pub(crate) fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid(_))
    }

    pub fn is_filtered(&self) -> bool {
        matches!(self, Self::Filtered(_))
    }

    pub fn is_normal(&self) -> bool {
        matches!(self, Self::Normal(_))
    }
}

/// Represents an index structure for managing duplicate groups and their associated metadata.
#[derive(Archive, Deserialize, Serialize)]
pub struct Index {
    /// A map of duplicate group keys to their associated read locations.
    groups: IndexMap<DuplicateGroupKey, SmallVec<[ReadLocation; 1]>>,

    /// Metadata associated with the index.
    metadata: IndexMetadata,

    /// The start time of the indexation process.
    #[rkyv(with = rkyv::with::AsUnixTime)]
    _start: std::time::SystemTime,

    /// The current path of the index and file. This should be set to the invocation path upon
    /// index loading, unlike metadata.file_path
    path: FileIndexPath,
}

impl Index {
    /// Creates a new `Index` instance with the given file path.
    pub fn new(file: FileIndexPath) -> Self {
        let mut index = Self {
            groups: IndexMap::new(),
            metadata: IndexMetadata::default(),
            _start: std::time::SystemTime::now(),
            path: file.clone(),
        };
        index.metadata.add_general_metadata(file);
        index
    }

    /// Retrieves a duplicate group by its record identifier.
    pub fn get_by_record_id(&self, id: RecordIdentifier) -> Option<DuplicateGroup> {
        self.get_by_key(&DuplicateGroupKey::Normal(id))
    }

    /// Adds a read to the index under the specified duplicate group key.
    pub fn add_read(&mut self, key: DuplicateGroupKey, read: ReadLocation, seq: SequenceRecord) {
        self.metadata.add_read_metadata(&key, seq);
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
    fn get_by_key(&self, key: &DuplicateGroupKey) -> Option<DuplicateGroup> {
        self.groups
            .get_full(key)
            .map(|(index, key, reads)| DuplicateGroup { key, reads, index })
    }

    /// Adds a read entry to the index under the specified duplicate group key.
    fn add_read_entry(&mut self, key: DuplicateGroupKey, read: ReadLocation) {
        self.groups
            .entry(key)
            .and_modify(|e| e.push(read.clone()))
            .or_insert_with(|| smallvec![read]);
    }
}

impl ArchivedIndex {
    /// Returns an iterator over the duplicate groups in the index.
    pub fn groups(&self) -> impl Iterator<Item = ArchivedDuplicateGroup> {
        self.groups
            .iter()
            .enumerate()
            .map(|(index, (key, value))| ArchivedDuplicateGroup {
                key,
                reads: value,
                id: index,
            })
    }

    /// Get an read accessor that can perform filesystem operations and read the original reads
    pub fn get_read_accessor(
        &self,
        index_path: &FileIndexPath,
    ) -> Result<Box<dyn GroupedReadsAccessor>> {
        // for now, all reads are uncompressed
        let is_zlib_compressed = false;
        if is_zlib_compressed {
            todo!()
        } else {
            let fastq_path = index_path.fastq();
            let reader = UncompressedFileReader::new(fastq_path)
                .with_context(|| format!("Error reading read file {}", fastq_path.display()))?;
            Ok(Box::new(reader))
        }
    }

    /// Provides access to the metadata of the index.
    pub fn metadata(&self) -> IndexMetadata {
        rkyv::deserialize::<IndexMetadata, rancor::Error>(&self.metadata)
            .expect("Failed to deserialize metadata")
    }
}
