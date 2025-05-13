use rkyv::{Archive, Deserialize, Serialize};
use std::fmt;

/// A RecordIdentifier is a store of the BC/UMI identifier of a read.
#[derive(Archive, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct RecordIdentifier(pub String);

impl RecordIdentifier {
    pub fn from_recs(v: &[&str]) -> Self {
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
