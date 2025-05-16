use needletail::parser::SequenceRecord;

use crate::io::index::ArchivedDuplicateGroupKey;
use crate::{cli::ConsensusArgs, io::index::ArchivedDuplicateGroup};
use std::fmt::Write as StrWrite;
use std::time::Instant;

use std::str;

/// A struct to format the header of a consensus sequence
pub struct HeaderFormatter<'a> {
    buffer: String,
    group: &'a ArchivedDuplicateGroup<'a>,
    args: &'a ConsensusArgs,
    key: String,
    len: usize,
    id: usize,
    start_time: Instant,
}

impl<'a> HeaderFormatter<'a> {
    pub fn new(group: &'a ArchivedDuplicateGroup, args: &'a ConsensusArgs) -> Self {
        let len = group.reads.len();
        let id = group.id;

        let buffer = String::new();

        let key = match group.key {
            ArchivedDuplicateGroupKey::Normal(id) => id.to_string(),
            _ => "todo".to_string(),
        };

        let start_time = Instant::now();

        let mut s = Self {
            buffer,
            group,
            args,
            key,
            len,
            id,
            start_time,
        };

        s._print_header_intro();
        s
    }

    fn _print_header_intro(&mut self) {
        let consensus_type = match self.group.key {
            ArchivedDuplicateGroupKey::Normal(_) => {
                if self.len == 1 {
                    "single".to_string()
                } else {
                    format!("consensus_from_{}", self.len)
                }
            }
            ArchivedDuplicateGroupKey::Invalid(_) => "ignored".to_string(),
            ArchivedDuplicateGroupKey::Filtered(_) => "filtered".to_string(),
        };

        write!(
            self.buffer,
            "{}|id={}|type={}",
            self.key, self.id, consensus_type
        )
        .unwrap();

        if self.args.report_original_header {
            write!(self.buffer, "|orig_header=[").unwrap();
        }
    }

    pub fn add_read(&mut self, read: &SequenceRecord) {
        if self.args.report_original_header {
            let id = str::from_utf8(read.id()).unwrap();
            write!(self.buffer, "\"{}\",", id.escape_default()).unwrap();
        }
    }

    pub fn finalize(&mut self) -> &str {
        if self.args.report_original_header {
            // truncate the final character to remove the extra ','
            self.buffer.truncate(self.buffer.len() - 1);
            write!(self.buffer, "]").unwrap();
        }

        if self.args.extra_stats {
            write!(
                self.buffer,
                "|elapsed_us={}",
                self.start_time.elapsed().as_micros()
            )
            .unwrap()
        }

        &self.buffer
    }

    pub fn make_original_header(
        &self,
        record: &SequenceRecord,
        read_idx: usize,
        alignment_result: &spoa::AlignmentResult,
    ) -> String {
        let read_pos = read_idx + 1;

        let mut params = format!(
            "{}|id={}.{read_pos}|type=orig_{read_pos}_of_{}",
            self.key, self.id, self.len
        );

        if self.args.report_original_header {
            write!(
                params,
                "|orig_header=[\"{}\"]",
                str::from_utf8(record.id()).unwrap().escape_default()
            )
            .unwrap();
        };

        if self.args.extra_stats {
            write!(
                params,
                "|align_new_nodes={}|align_sequence_len={}|align_valid_nodes={}",
                alignment_result.new_nodes,
                alignment_result.sequence_len,
                alignment_result.valid_nodes
            )
            .unwrap();
        }

        params
    }
}
