use rkyv::{Archive, Deserialize, Serialize};
use std::fmt;

/// A RecordIdentifier is a store of the BC/UMI identifier of a read.
#[derive(Archive, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct RecordIdentifier(String);

impl RecordIdentifier {
    pub fn from_recs<'a>(v: &[&str]) -> Self {
        Self(v.join("_"))
    }

    pub fn from_str(v: &str) -> Self {
        Self(v.to_string())
    }
}

impl fmt::Display for RecordIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for ArchivedRecordIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// #[derive(Serialize, Debug, Archive)]
// pub struct DuplicateStatistics {
//     pub total_reads: usize,
//     pub duplicate_reads: usize,
//     pub duplicate_ids: usize,
//     pub proportion_duplicate: f64,
//     pub distribution: BTreeMap<usize, usize>,
// }
