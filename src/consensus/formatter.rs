use needletail::parser::SequenceRecord;

use crate::cli::ConsensusArgs;
use crate::io::index::{DuplicateGroupType, DuplicateGroup};
use std::fmt::Write as StrWrite;
use std::time::Instant;

use std::str;

/// A struct to format the header of a consensus sequence
pub struct HeaderFormatter<'a> {
    args: &'a ConsensusArgs,
    key: String,
    id: usize,
    start_time: Instant,
    consensus_type: String,
    orig_headers: Vec<Vec<String>>,
}

impl<'a> HeaderFormatter<'a> {
    pub fn new(group: &'a DuplicateGroup, args: &'a ConsensusArgs) -> Self {
        let group_size = group.reads.len();
        let id = group.id;
        let key = group.key.to_string();

        let start_time = Instant::now();
        let consensus_type = match group.group_type {
            DuplicateGroupType::Valid => {
                if group_size == 1 {
                    "single".to_string()
                } else {
                    "consensus".to_string()
                }
            }
            DuplicateGroupType::Filtered => "filtered".to_string(),
        };

        // placeholder
        let orig_headers = vec![Vec::new()];

        Self {
            args,
            key,
            id,
            start_time,
            consensus_type,
            orig_headers,
        }
    }

    pub fn add_read(&mut self, cluster_id: usize, read: &SequenceRecord) {
        let headers_len = self.orig_headers.len();

        let orig_headers = match self.orig_headers.get_mut(cluster_id) {
            // if the cluster already exists
            Some(v) => v,

            // if a new cluster is needed
            None if cluster_id == headers_len => {
                self.orig_headers.push(Vec::new());
                &mut self.orig_headers[headers_len]
            }

            // this should never occur...
            _ => panic!(
                "Index {} is more than one past the end of the vec (len = {})",
                cluster_id, headers_len
            ),
        };

        if self.args.report_original_header {
            let id = String::from_utf8(read.id().to_vec()).unwrap();
            orig_headers.push(format!("\"{}\"", id.escape_default()))
        } else {
            // we can do anything
            orig_headers.push(String::new())
        }
    }

    pub fn make_consensus_header(&self, cluster_id: usize) -> String {
        let mut header = String::new();
        let orig_headers = &self.orig_headers[cluster_id];

        // base
        write!(
            header,
            "{}|id={}|type={}",
            self.key, self.id, self.consensus_type,
        )
        .unwrap();

        // if this is a consensus read AND clusters are enabled, report cluster count
        if !self.args.no_clustering {
            let cluster_cnt = cluster_id + 1;
            write!(header, "|cluster={cluster_cnt}").unwrap()
        }

        write!(header, "|reads_called={}", orig_headers.len()).unwrap();

        // report extra stats
        if self.args.extra_stats {
            write!(
                header,
                "|elapsed_us={}",
                self.start_time.elapsed().as_micros()
            )
            .unwrap()
        }

        // report original header
        if self.args.report_original_header {
            write!(header, "|orig_header=[{}]", orig_headers.join(",")).unwrap()
        }

        header
    }

    pub fn make_original_header(
        &self,
        record: &SequenceRecord,
        read_idx: usize,
        alignment_predictions: Vec<spoa::AlignmentResult>,
        cluster_id: usize,
    ) -> String {
        let read_pos = read_idx + 1;

        let mut header = String::new();

        // format base
        write!(
            header,
            "{}|id={}|type=original|read={}",
            self.key, self.id, read_pos
        )
        .unwrap();

        if !self.args.no_clustering {
            let cluster_cnt = cluster_id + 1;
            write!(header, "|cluster={cluster_cnt}").unwrap()
        }

        if self.args.extra_stats {
            for aln in alignment_predictions.iter() {
                write!(
                    header,
                    "|align_new_nodes={}|align_sequence_len={}|align_valid_nodes={}",
                    aln.new_nodes, aln.sequence_len, aln.valid_nodes
                )
                .unwrap();
            }
        }

        if self.args.report_original_header {
            write!(
                header,
                "|orig_header=[\"{}\"]",
                str::from_utf8(record.id()).unwrap().escape_default()
            )
            .unwrap();
        };

        header
    }
}
