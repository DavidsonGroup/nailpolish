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
    group_size: usize,
    id: usize,
    start_time: Instant,
    consensus_type: String,
    orig_header: Vec<String>,
}

impl<'a> HeaderFormatter<'a> {
    pub fn new(group: &'a ArchivedDuplicateGroup, args: &'a ConsensusArgs) -> Self {
        let group_size = group.reads.len();
        let id = group.id;

        let buffer = String::new();

        let key = match group.key {
            ArchivedDuplicateGroupKey::Normal(id) => id.to_string(),
            _ => "todo".to_string(),
        };

        let start_time = Instant::now();
        let consensus_type = match group.key {
            ArchivedDuplicateGroupKey::Normal(_) => {
                if group_size == 1 {
                    "single".to_string()
                } else {
                    format!("consensus_from_{}", group_size)
                }
            }
            ArchivedDuplicateGroupKey::Invalid(_) => "ignored".to_string(),
            ArchivedDuplicateGroupKey::Filtered(_) => "filtered".to_string(),
        };

        // placeholder
        let orig_header = Vec::new();

        let mut s = Self {
            buffer,
            group,
            args,
            key,
            group_size,
            id,
            start_time,
            consensus_type,
            orig_header,
        };

        s
    }

    pub fn add_read(&mut self, read: &SequenceRecord) {
        if self.args.report_original_header {
            let id = String::from_utf8(read.id().to_vec()).unwrap();
            self.orig_header
                .push(format!("\"{}\"", id.escape_default()))
        }
    }

    pub fn make_consensus_header(&self, cluster_id: usize) -> String {
        let mut header = String::new();

        // base
        write!(
            header,
            "{}|id={}|type={}",
            self.key, self.id, self.consensus_type
        )
        .unwrap();

        // if this is a consensus read AND clusters are enabled, report cluster count
        if self.group_size > 1 && !self.args.disable_clusters {
            let cluster_cnt = cluster_id + 1;
            write!(header, "|cluster={cluster_cnt}").unwrap()
        }

        // report original header
        if self.args.report_original_header {
            write!(header, "|orig_header=[{}]", self.orig_header.join(",")).unwrap()
        }

        // report extra stats
        if self.args.extra_stats {
            write!(
                header,
                "|elapsed_us={}",
                self.start_time.elapsed().as_micros()
            )
            .unwrap()
        }

        header
    }

    pub fn make_original_header(
        &self,
        record: &SequenceRecord,
        read_idx: usize,
        alignment_result: &spoa::AlignmentResult,
    ) -> String {
        let read_pos = read_idx + 1;

        let mut header = String::new();

        // format base
        write!(
            header,
            "{}|id={}.{read_pos}|type=orig_{read_pos}_of_{}",
            self.key, self.id, self.group_size
        )
        .unwrap();

        if self.args.report_original_header {
            write!(
                header,
                "|orig_header=[\"{}\"]",
                str::from_utf8(record.id()).unwrap().escape_default()
            )
            .unwrap();
        };

        if self.args.extra_stats {
            write!(
                header,
                "|align_new_nodes={}|align_sequence_len={}|align_valid_nodes={}",
                alignment_result.new_nodes,
                alignment_result.sequence_len,
                alignment_result.valid_nodes
            )
            .unwrap();
        }

        header
    }
}
