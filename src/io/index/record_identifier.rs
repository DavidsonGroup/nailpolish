// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::fmt;

use anyhow::{Context, Result};
use rkyv::{Archive, Deserialize, Serialize};

/// A RecordIdentifier is a store of the BC/UMI identifier of a read.
#[derive(Archive, Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Debug)]
pub struct RecordIdentifier(pub String);

impl RecordIdentifier {
    pub fn from_recs(v: &[&str]) -> Self {
        Self(v.join("\0"))
    }

    pub fn from_str(v: &str) -> Self {
        Self(v.to_string())
    }

    pub fn components(&self) -> std::str::Split<'_, char> {
        self.0.split('\0')
    }

    /// Return the N-th component, if present
    pub fn component(&self, idx: usize) -> Option<&str> {
        self.components().nth(idx)
    }
}

impl ArchivedRecordIdentifier {
    pub fn components(&self) -> std::str::Split<'_, char> {
        self.0.split('\0')
    }

    /// Return the N-th component, if present
    pub fn component(&self, idx: usize) -> Result<&str> {
        self.components()
            .nth(idx)
            .context("Component does not exist")
    }
}

impl fmt::Display for RecordIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.replace('\0', "_"))
    }
}

impl fmt::Display for ArchivedRecordIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.replace('\0', "_"))
    }
}
