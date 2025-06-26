/// Read filtering based on length and quality criteria
use crate::{
    cli::interval::ArgInterval,
    io::index::{DuplicateGroup, DuplicateGroupLocation, DuplicateGroupType, ReadLocation},
};
use smallvec::{smallvec, SmallVec};

/// Options for filtering reads based on length and quality
pub struct FilterOpts {
    /// Length interval filter
    pub len: ArgInterval,
    /// Quality score interval filter
    pub quality: ArgInterval,
    /// Max group size filter
    pub max_group_size: usize,
}

impl FilterOpts {
    /// Creates new filter options from command line arguments
    pub fn new(cli: &crate::cli::ConsensusArgs) -> Self {
        Self {
            len: cli.len,
            quality: cli.qual,
            max_group_size: cli.max_group_size,
        }
    }
}

/// Determines if a read should be kept based on its length and quality
pub fn should_keep(loc: &ReadLocation, opts: &FilterOpts) -> bool {
    let quality_good = opts.quality.contains(loc.qual);
    let len_good = opts.len.contains(loc.byte_len() as f32);

    quality_good && len_good
}

pub fn filter_group_locations(
    group: &DuplicateGroupLocation,
    reads: Vec<Vec<u8>>,
    opts: &FilterOpts,
) -> Vec<DuplicateGroup> {
    if group.reads.len() > opts.max_group_size {
        // this group should be filtered out
        let id = group.id;
        let key = group.key.clone();

        reads
            .into_iter()
            .map(|r| DuplicateGroup {
                id,
                key: key.clone(),
                reads: vec![r],
                group_type: DuplicateGroupType::Filtered,
            })
            .collect()
    } else {
        // For normal-sized groups, filter individual reads based on should_keep criteria
        let mut kept_reads = Vec::new();
        let mut filtered_reads = Vec::new();

        let id = group.id;
        let key = group.key.clone();

        for (read_loc, read) in group.reads.iter().zip(reads) {
            if should_keep(read_loc, opts) {
                kept_reads.push(read.clone())
            } else {
                filtered_reads.push(read.clone())
            }
        }

        let mut groups = Vec::new();
        // Add the original group with kept reads (if any)
        if !kept_reads.is_empty() {
            groups.push(DuplicateGroup {
                id,
                key: key.clone(),
                reads: kept_reads,
                group_type: DuplicateGroupType::Valid,
            });
        }

        groups.extend(filtered_reads.into_iter().map(|r| DuplicateGroup {
            id,
            key: key.clone(),
            reads: vec![r],
            group_type: DuplicateGroupType::Filtered,
        }));

        groups
    }
}
