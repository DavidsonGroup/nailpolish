// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

pub mod comment;
use comment::{write_comments, AlignmentResultSerialWrapper, FastqComment, FastqCommentType};
use serde_json::json;

use std::time::Instant;

use crate::{
    cli::ConsensusArgs,
    consensus::Cluster,
    io::index::{DuplicateGroup, DuplicateGroupType},
};

// Nailpolish .fastq comment specification
//
// Nailpolish will insert 'fastq comments' into the .fastq header, which are formatted
// according to the SAM specification: https://samtools.github.io/hts-specs/SAMtags.pdf
// A typical Nailpolish read will look like this:
//
//   consensus_252    MI:Z:ATCGATCG_TCGATCGA   XT:Z:consensus
//
// The SAM tag types can be found in the SAM specification.
// For complex data types (i.e. FastqCommentType::JSONObject), the result will be stored
// as a JSON object encoded into string form. Newlines and tab characters will be stripped
// and converted to their textual equivalent i.e. `\t` and `\n`.
//
// Below are the types of tags which can be found. All non-standard (according to the SAM) spec
// tags are of the format nX, where X is a capital alphabetical letter.
// Some of these tags will require parameters (such as `--extra-stats`) to be passed in.
//
// MI:Z:    Molecular barcode. All independent components of the barcode are combined using the _ character.
//          For instance, a read with a molecular barcode and unique molecular identifier will be stored in
//          the form BC_UMI.
// nI:i:    Duplicate group index in integer form. All groups with an identical `MI` will have an identical `nI`.
// nT:Z:    Read type. Can be one of `consensus`, `original`, `filtered`, `simplex`.
// nC:I:    Duplicate /cluster/ index. A duplicate group can contain multiple clusters within it. Clusters are
//          1-indexed within each duplicate group. This tag will not be present if clustering is not enabled.
// nL:I:    Cluster length. If clustering is disabled, this is instead the duplicate group length. The number of reads
//          within that cluster or group.
// nT:i:    Elapsed time to process duplicate group, in microseconds
// nR:i:    Reported in original reads to show the read index in the duplicate group
// nH:Z:    Original headers of the component reads. This is an array, formatted in JSON array style. Only produced if
//          `--report-original-headers` is given.
// nA:Z:    Alignment results, as a JSON array of AlignmentResult objects
//

/// A struct to format the header of a consensus sequence
pub struct HeaderFormatter<'a> {
    args: &'a ConsensusArgs,
    group_id: usize,
    start_time: Instant,
    consensus_type: String,
    global_comments: Vec<FastqComment>,
}

impl<'a> HeaderFormatter<'a> {
    pub fn new(group: &'a DuplicateGroup, args: &'a ConsensusArgs) -> Self {
        let group_size = group.reads.len();
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

        let tags = vec![
            FastqComment::new("MI", FastqCommentType::String(key.clone())),
            FastqComment::new("nI", FastqCommentType::Integer(group.id)),
        ];

        Self {
            args,
            group_id: group.id,
            start_time,
            consensus_type,
            global_comments: tags,
        }
    }

    pub fn make_filtered_header(&self, read_idx: usize) -> String {
        let comments = vec![FastqComment::new(
            "nT",
            FastqCommentType::String("filtered".to_string()),
        )];

        let mut result = format!("filtered_{}_{}", self.group_id, read_idx);

        write_comments(&mut result, &self.global_comments);
        write_comments(&mut result, &comments);

        result
    }

    pub fn make_simplex_header(&self, orig_header: String) -> String {
        let mut read_comments = vec![];

        read_comments.push(FastqComment::new(
            "nT",
            FastqCommentType::String("simplex".to_string()),
        ));

        // report original header
        if self.args.report_original_header {
            read_comments.push(FastqComment::new(
                "nH",
                FastqCommentType::JSONObject(json!(vec![orig_header])),
            ));
        }

        let mut header = format!("processed_{}_{}", self.group_id, 1);

        write_comments(&mut header, &self.global_comments);
        write_comments(&mut header, &read_comments);

        header
    }

    pub fn make_consensus_header(&self, cluster: &Cluster) -> String {
        let mut read_comments = vec![];

        read_comments.push(FastqComment::new(
            "nT",
            FastqCommentType::String(self.consensus_type.clone()),
        ));

        // if this is a consensus read AND clusters are enabled, report cluster count
        if !self.args.no_clustering {
            read_comments.push(FastqComment::new(
                "nC",
                FastqCommentType::Integer(cluster.id),
            ));
        }

        read_comments.push(FastqComment::new(
            "nL",
            FastqCommentType::Integer(cluster.read_count),
        ));

        // report extra stats
        if self.args.extra_stats {
            read_comments.push(FastqComment::new(
                "nE",
                FastqCommentType::Integer(self.start_time.elapsed().as_micros() as usize),
            ))
        }

        // report original header
        if self.args.report_original_header {
            read_comments.push(FastqComment::new(
                "nH",
                FastqCommentType::JSONObject(json!(cluster.orig_read_headers)),
            ));
        }

        let mut header = format!("processed_{}_{}", self.group_id, cluster.id);

        write_comments(&mut header, &self.global_comments);
        write_comments(&mut header, &read_comments);

        header
    }

    pub fn make_original_header(
        &self,
        read_idx: usize,
        alignment_predictions: Vec<spoa::AlignmentResult>,
        cluster: &Cluster,
    ) -> String {
        let mut read_comments = vec![];

        read_comments.push(FastqComment::new(
            "nT",
            FastqCommentType::String("original".to_string()),
        ));

        if !self.args.no_clustering {
            read_comments.push(FastqComment::new(
                "nC",
                FastqCommentType::Integer(cluster.id),
            ));
        }

        read_comments.push(FastqComment::new("nR", FastqCommentType::Integer(read_idx)));

        if self.args.extra_stats {
            let value = json!(alignment_predictions
                .into_iter()
                .map(|v| AlignmentResultSerialWrapper(v))
                .collect::<Vec<_>>());

            read_comments.push(FastqComment::new("nA", FastqCommentType::JSONObject(value)));
        }

        let mut header = format!("original_{}_{}_{}", self.group_id, cluster.id, read_idx);

        write_comments(&mut header, &self.global_comments);
        write_comments(&mut header, &read_comments);

        header
    }
}
