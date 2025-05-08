use crate::io::index::ArchivedDuplicateGroupKey;
use crate::{cli::ConsensusArgs, io::index::ArchivedDuplicateGroup};

use bio::io::fastq::Record;

pub fn make_consensus_header(
    group: &ArchivedDuplicateGroup,
    reads: &Vec<Record>,
    args: &ConsensusArgs,
) -> String {
    let len = group.reads.len();
    let id = group.index;
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
