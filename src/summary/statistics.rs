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

/// Implement the `Display` trait for `RecordIdentifier`. This allows a RecordIdentifier to
/// be converted to a string through `.to_string()` or using format macros. See `.from_string()`
/// for the inverse function.
impl std::fmt::Display for ArchivedRecordIdentifier {
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

#[derive(Serialize, Debug, Archive)]
pub struct DuplicateStatistics {
    pub total_reads: usize,
    pub duplicate_reads: usize,
    pub duplicate_ids: usize,
    pub proportion_duplicate: f64,
    pub distribution: BTreeMap<usize, usize>,
}
