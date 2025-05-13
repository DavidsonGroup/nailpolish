use crate::io::index::ArchivedDuplicateGroupKey;
use crate::{cli::ConsensusArgs, io::index::ArchivedDuplicateGroup};
use std::fmt::Write as StrWrite;

use bio::io::fastq::Record;

pub fn make_consensus_header(
    group: &ArchivedDuplicateGroup,
    reads: &Vec<Record>,
    args: &ConsensusArgs,
) -> String {
    let len = group.reads.len();
    let id = group.id;
    let single = len == 1;

    let type_ = match group.key {
        ArchivedDuplicateGroupKey::Normal(_) => {
            if single {
                "single".to_string()
            } else {
                format!("consensus_from_{}", len)
            }
        }
        ArchivedDuplicateGroupKey::Invalid(_) => "ignored".to_string(),
        ArchivedDuplicateGroupKey::Filtered(_) => "filtered".to_string(),
    };

    let key = match group.key {
        ArchivedDuplicateGroupKey::Normal(id) => id.to_string(),
        _ => "TEST".to_string(),
    };

    let mut params = format!("{key}|id={id}|type={type_}");

    if args.report_original_header {
        params.push_str("|orig_header=");

        for r in reads {
            params.push_str("[\"");
            params.extend(r.id().escape_default());
            params.push_str("\",");
        }

        // truncate the final character to remove the extra ','
        params.truncate(params.len() - 1);
        params.push(']');
    };

    params
}

/// Generates a header string for an original read in a duplicate group.
/// Includes original header if requested in args.
/// `read_idx` should be 0-indexed.
pub fn make_original_header(
    group: &ArchivedDuplicateGroup,
    read: &Record,
    read_idx: usize,
    args: &ConsensusArgs,
) -> String {
    let id = group.id;
    let size = group.reads.len();
    let read_pos = read_idx + 1;

    let key = match group.key {
        ArchivedDuplicateGroupKey::Normal(id) => id.to_string(),
        _ => todo!(),
    };

    let mut params = format!("{key}|id={id}.{read_pos}|type=orig_{read_pos}_of_{size}");

    if args.report_original_header {
        write!(params, "|orig_header={}", read.id()).unwrap();
    };

    params
}
