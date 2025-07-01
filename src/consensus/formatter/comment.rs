// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use itertools::Itertools;
use serde::Serialize;
use spoa::AlignmentResult;
use std::fmt;

#[derive(Serialize)]
pub struct AlignmentResultSerialWrapper(
    #[serde(with = "SerializableAlignmentResult")] pub AlignmentResult,
);

#[derive(serde::Serialize)]
#[serde(remote = "AlignmentResult")]
struct SerializableAlignmentResult {
    new_nodes: u32,
    sequence_len: u32,
    valid_nodes: u32,
}

impl From<AlignmentResult> for SerializableAlignmentResult {
    fn from(value: AlignmentResult) -> Self {
        Self {
            new_nodes: value.new_nodes,
            sequence_len: value.sequence_len,
            valid_nodes: value.valid_nodes,
        }
    }
}

/// This is .fastq comment, which is identical to a
/// SAM tag: https://samtools.github.io/hts-specs/SAMtags.pdf
/// minimap2 will copy comments to output as SAM tags
/// if the `-y` option is given.
pub struct FastqComment {
    /// The tag/name of the comment
    pub tag: String,

    /// The value stored within the comment
    pub value: FastqCommentType,
}

impl FastqComment {
    pub fn new(tag: &str, value: FastqCommentType) -> Self {
        Self {
            tag: tag.to_string(),
            value,
        }
    }
}

#[allow(dead_code)]
pub enum FastqCommentType {
    /// Formats as a tab-escaped string :Z:
    String(String),
    /// Formats as an integer :i:
    Integer(usize),
    /// Formats as a 'real number' :f:
    RealNumber(f64),
    /// Formats into a serialised JSON representation of the input,
    /// stored as a tab-escaped text string :Z:
    JSONObject(serde_json::Value),
}

impl fmt::Display for FastqComment {
    // implement to_string() and the Display trait
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tag = &self.tag;

        match &self.value {
            FastqCommentType::String(v) => {
                let v = v.replace("\t", "\\t").replace("\n", "\\n");
                write!(f, "{tag}:Z:{v}")
            }
            FastqCommentType::Integer(v) => write!(f, "{tag}:i:{v}"),
            FastqCommentType::RealNumber(v) => write!(f, "{tag}:f:{v}"),
            FastqCommentType::JSONObject(items) => {
                let display = items.to_string().replace("\t", "\\t").replace("\n", "\\n");
                write!(f, "{tag}:Z:{display}")
            }
        }
    }
}

pub fn write_comments(w: &mut impl std::fmt::Write, comments: &Vec<FastqComment>) {
    write!(w, "\t{}", comments.iter().map(|v| v.to_string()).join("\t")).unwrap()
}
