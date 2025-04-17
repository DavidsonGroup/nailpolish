use crate::io::InMemoryRecord;
use crate::r#mod::{IndexReader, IndexRecord};
use anyhow::{ensure, Context, Result};
use indexmap::IndexMap;
use rkyv::{Archive, Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Index;
use std::rc::Rc;
use std::sync::Arc;

/// A struct representing the position of a record.
///
/// # Fields
///
/// * `pos` - The position of the record in the input file
/// * `length` - The length of the record, in bytes
#[derive(Copy, Clone)]
pub struct RecordPosition {
    pub pos: usize,
    pub length: usize,
}
// pub type DuplicateMap = IndexMap<RecordIdentifier, Vec<RecordPosition>>;

pub struct DuplicateMap {
    pub by_id: IndexMap<RecordIdentifier, Vec<RecordPosition>>,
    pub pos_to_id: IndexMap<usize, RecordIdentifier>,
}

impl DuplicateMap {
    pub fn new() -> Self {
        DuplicateMap {
            by_id: Default::default(),
            pos_to_id: Default::default(),
        }
    }

    pub fn insert(&mut self, record: &IndexRecord) {
        let id = RecordIdentifier::from_string(&record.id);

        let rec_pos = RecordPosition {
            pos: record.pos,
            length: record.rec_len,
        };

        self.pos_to_id.insert(record.pos, id.clone());

        self.by_id
            .entry(id)
            .and_modify(|e| e.push(rec_pos))
            .or_insert(vec![rec_pos]);
    }

    pub fn shrink_to_fit(&mut self) {
        self.by_id.shrink_to_fit();
        self.pos_to_id.shrink_to_fit();
    }

    pub fn records_by_id(&self, id: &RecordIdentifier) -> Option<&Vec<RecordPosition>> {
        self.by_id.get(id)
    }

    pub fn records_by_pos(&self, pos: &usize) -> Option<&Vec<RecordPosition>> {
        let id = self.pos_to_id.get(pos)?;
        self.records_by_id(id)
    }
}

/// A struct representing a record identifier with a head and a tail.
///
/// # Fields
///
/// * `head` - The head part of the record identifier.
/// * `tail` - The tail part of the record identifier.
#[derive(Eq, PartialEq, Hash, Debug, Archive, Serialize, Deserialize, Clone)]
pub struct RecordIdentifier {
    pub head: String,
    pub tail: String,
}

/// Implement the `Display` trait for `RecordIdentifier`. This allows a RecordIdentifier to
/// be converted to a string through `.to_string()` or using format macros. See `.from_string()`
/// for the inverse function.
impl std::fmt::Display for RecordIdentifier {
    /// Format the `RecordIdentifier` as a string.
    ///
    /// If the `tail` is empty, only the `head` is returned.
    /// Otherwise, the `head` and `tail` are concatenated with an underscore.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.tail.is_empty() {
            f.write_str(&self.head)
        } else {
            write!(f, "{}_{}", self.head, self.tail)
        }
    }
}

impl RecordIdentifier {
    /// Creates a `RecordIdentifier` from a string slice. See `.to_string()` for the inverse
    /// function.
    ///
    /// # Arguments
    ///
    /// * `s` - A string slice that holds the record identifier.
    ///
    /// # Returns
    ///
    /// A `RecordIdentifier` with the head and tail parts extracted from the input string.
    pub fn from_string(s: &str) -> Self {
        let split_loc = match s.find('_') {
            Some(v) => v,
            None => s.len() - 1,
        };

        RecordIdentifier {
            head: s[..split_loc].to_string(),
            tail: s[(split_loc + 1)..].to_string(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct DuplicateStatistics {
    pub total_reads: usize,
    pub duplicate_reads: usize,
    pub duplicate_ids: usize,
    pub proportion_duplicate: f64,
    pub distribution: BTreeMap<usize, usize>,
}

impl IndexReader {
    /// Reads a FASTQ index file and identifies duplicate records.
    ///
    /// # Arguments
    ///
    /// * `index` - A string slice that holds the path to the index file.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - `DuplicateMap`: A map of `RecordIdentifier` to a vector of indices where duplicates are found.
    /// - `DuplicateStatistics`: Statistics about the duplicates found.
    /// - `FastqFile`: Metadata about the FASTQ file.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file cannot be opened or read, or if the file format is incorrect.
    pub fn get_duplicates(&mut self) -> Result<(DuplicateMap, DuplicateStatistics)> {
        info!("Reading index file...");

        let mut map = DuplicateMap::new();

        let mut stats = DuplicateStatistics {
            total_reads: 0,
            duplicate_reads: 0,
            duplicate_ids: 0,
            proportion_duplicate: 0.0,
            distribution: BTreeMap::new(),
        };

        // Parse each row of the reader
        for read in self.index_records()? {
            let record: IndexRecord = read?;
            if record.ignored {
                continue;
            }

            stats.total_reads += 1;

            map.insert(&record);
        }

        map.shrink_to_fit(); // optimise memory usage

        // Compute information about the duplicates
        stats.duplicate_ids = 0;
        stats.duplicate_reads = map
            .by_id
            .values()
            .map(|v| {
                let length = v.len();
                if length > 1 {
                    stats.duplicate_ids += 1;

                    if let Some(x) = stats.distribution.get_mut(&length) {
                        *x += 1
                    } else {
                        stats.distribution.insert(length, 1);
                    }
                    length
                } else {
                    0
                }
            })
            .sum();

        stats
            .distribution
            .insert(1, stats.total_reads - stats.duplicate_reads);

        stats.proportion_duplicate = stats.duplicate_reads as f64 / stats.total_reads as f64;

        info!("Generated duplicate map from index file");

        Ok((map, stats))
    }
}
