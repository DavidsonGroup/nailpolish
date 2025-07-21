// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

/// Read filtering based on length and quality criteria
use crate::{
    cli::interval::ArgInterval,
    io::index::{DuplicateGroup, DuplicateGroupLocation, DuplicateGroupType, ReadLocation},
};

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

pub fn group_from_simplex(group: &DuplicateGroupLocation, reads: Vec<Vec<u8>>) -> DuplicateGroup {
    DuplicateGroup {
        id: group.id,
        key: group.key.clone(),
        reads,
        group_type: DuplicateGroupType::Valid,
    }
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

        vec![DuplicateGroup {
            id,
            key,
            reads,
            group_type: DuplicateGroupType::Filtered,
        }]
    } else {
        let id = group.id;
        let key = group.key.clone();

        // For normal-sized groups, filter individual reads based on should_keep criteria
        let mut valid_group = DuplicateGroup {
            id,
            key: key.clone(),
            reads: vec![],
            group_type: DuplicateGroupType::Valid,
        };
        let mut filt_group = DuplicateGroup {
            id,
            key: key,
            reads: vec![],
            group_type: DuplicateGroupType::Filtered,
        };

        for (read_loc, read) in group.reads.iter().zip(reads) {
            if should_keep(read_loc, opts) {
                valid_group.reads.push(read)
            } else {
                filt_group.reads.push(read)
            }
        }

        let mut groups = vec![];

        if valid_group.reads.len() > 0 {
            groups.push(valid_group);
        }
        if filt_group.reads.len() > 0 {
            groups.push(filt_group);
        }

        groups
    }
}
