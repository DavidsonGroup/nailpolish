// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

/// Read filtering based on length and quality criteria
use std::cmp::Reverse;

use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

use crate::{
    cli::{interval::ArgInterval, LargeGroupMethod},
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
    /// How to handle groups exceeding max_group_size
    pub large_group_method: LargeGroupMethod,
}

impl FilterOpts {
    /// Creates new filter options from command line arguments
    pub fn new(cli: &crate::cli::ConsensusArgs) -> Self {
        Self {
            len: cli.len,
            quality: cli.qual,
            max_group_size: cli.max_group_size,
            large_group_method: cli.large_group_method.clone(),
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
    let id = group.id;
    let key = group.key.clone();
    let is_large = group.reads.len() > opts.max_group_size;

    // Handle early-exit cases for large groups
    if is_large {
        match opts.large_group_method {
            LargeGroupMethod::Drop => return vec![],
            LargeGroupMethod::Passthrough => {
                return vec![DuplicateGroup {
                    id,
                    key,
                    reads,
                    group_type: DuplicateGroupType::Filtered,
                }];
            }
            _ => {}
        }
    }

    // For large Sample/Longest groups, build a subsampled index set.
    // For normal groups, include all indices.
    let indices: Vec<usize> = if is_large {
        match opts.large_group_method {
            LargeGroupMethod::Sample => {
                // seed from group ID for fully reproducible output
                let mut rng = StdRng::seed_from_u64(id as u64);
                let mut idx: Vec<usize> = (0..reads.len()).collect();
                idx.shuffle(&mut rng);
                idx.truncate(opts.max_group_size);
                idx
            }
            LargeGroupMethod::Longest => {
                let mut idx: Vec<usize> = (0..reads.len()).collect();
                idx.sort_by_key(|&i| Reverse(group.reads[i].byte_len()));
                idx.truncate(opts.max_group_size);
                idx
            }
            _ => unreachable!(), // Drop and Passthrough already returned above
        }
    } else {
        (0..reads.len()).collect()
    };

    let mut valid_group = DuplicateGroup {
        id,
        key: key.clone(),
        reads: vec![],
        group_type: DuplicateGroupType::Valid,
    };
    let mut filt_group = DuplicateGroup {
        id,
        key,
        reads: vec![],
        group_type: DuplicateGroupType::Filtered,
    };

    for &i in &indices {
        if should_keep(&group.reads[i], opts) {
            valid_group.reads.push(reads[i].clone());
        } else {
            filt_group.reads.push(reads[i].clone());
        }
    }

    let mut groups = vec![];
    if !valid_group.reads.is_empty() {
        groups.push(valid_group);
    }
    if !filt_group.reads.is_empty() {
        groups.push(filt_group);
    }
    groups
}
