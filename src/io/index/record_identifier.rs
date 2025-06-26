// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::fmt;

use rkyv::{Archive, Deserialize, Serialize};

/// A RecordIdentifier is a store of the BC/UMI identifier of a read.
#[derive(Archive, Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Debug)]
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
