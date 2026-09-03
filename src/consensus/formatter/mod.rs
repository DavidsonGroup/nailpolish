// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

pub mod comment;
use comment::{write_comments, AlignmentResultSerialWrapper, FastqComment, FastqCommentType};
use rkyv::option::ArchivedOption;
use serde_json::json;

use std::time::Instant;

use crate::{
    cli::ConsensusArgs,
    consensus::{ArchivedCaptures, Cluster},
    io::index::DuplicateGroup,
};

// Nailpolish .fastq header specification
//
// The read name states the type of the read, followed by the indices which identify it:
//
//   singleton_{group}                  a duplicate group containing a single read
//   consensus_{group}_{cluster}        a consensus sequence called from a cluster of reads
//   passthrough_{group}_{read}         a read from a filtered group, emitted unchanged
//   original_{group}_{cluster}_{read}  a component read, with `--report-original-reads`
//
// Groups are 0-indexed; clusters and reads are 1-indexed within their parent. Clusters are
// only meaningfully distinct when clustering is enabled; otherwise the cluster index is always 1.
//
// Nailpolish will also insert 'fastq comments' into the .fastq header, which are formatted
// according to the SAM specification: https://samtools.github.io/hts-specs/SAMtags.pdf
// A typical Nailpolish read will look like this:
//
//   consensus_252_1    CB:Z:ATCGATCG   UB:Z:TCGATCGA   nL:i:3
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
// First, one tag per barcode component, named after the corresponding capture group in the
// barcode pattern — conventionally `CB:Z:` for the cell barcode and `UB:Z:` for the UMI.
//
// nL:I:    Cluster length. If clustering is disabled, this is instead the duplicate group length. The number of reads
//          within that cluster or group.
// nE:i:    Elapsed time to process duplicate group, in microseconds
// nH:Z:    Original headers of the component reads. This is an array, formatted in JSON array style. Only produced if
//          `--report-original-headers` is given.
// nA:Z:    Alignment results, as a JSON array of AlignmentResult objects
//

/// A struct to format the header of a consensus sequence
pub struct HeaderFormatter<'a> {
    args: &'a ConsensusArgs,
    group_id: usize,
    start_time: Instant,
    global_comments: Vec<FastqComment>,
}

impl<'a> HeaderFormatter<'a> {
    pub fn new(
        group: &'a DuplicateGroup,
        args: &'a ConsensusArgs,
        captures: &'a ArchivedCaptures,
    ) -> Self {
        let start_time = Instant::now();

        let mut tags = vec![];

        // add captures as tags
        for (tag, val) in captures.iter().zip(group.key.components()) {
            if let ArchivedOption::Some(tag) = tag {
                tags.push(FastqComment::new(
                    &tag,
                    FastqCommentType::String(val.to_string()),
                ))
            }
        }

        Self {
            args,
            group_id: group.id,
            start_time,
            global_comments: tags,
        }
    }

    pub fn make_passthrough_header(&self, read_idx: usize) -> String {
        let mut result = format!("passthrough_{}_{}", self.group_id, read_idx);

        write_comments(&mut result, &self.global_comments);

        result
    }

    pub fn make_singleton_header(&self, orig_header: String) -> String {
        let mut read_comments = vec![];

        // report original header
        if self.args.report_original_header {
            read_comments.push(FastqComment::new(
                "nH",
                FastqCommentType::JSONObject(json!(vec![orig_header])),
            ));
        }

        let mut header = format!("singleton_{}", self.group_id);

        write_comments(&mut header, &self.global_comments);
        write_comments(&mut header, &read_comments);

        header
    }

    pub fn make_consensus_header(&self, cluster: &Cluster) -> String {
        let mut read_comments = vec![FastqComment::new(
            "nL",
            FastqCommentType::Integer(cluster.read_count),
        )];

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

        let mut header = format!("consensus_{}_{}", self.group_id, cluster.id);

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
