/// Read filtering based on length and quality criteria
use crate::{cli::interval::ArgInterval, io::reads::record::QualityCompute};
use needletail::parser::SequenceRecord;

/// Options for filtering reads based on length and quality
pub struct FilterOpts {
    /// Length interval filter
    pub len: ArgInterval,
    /// Quality score interval filter
    pub quality: ArgInterval,
}

impl FilterOpts {
    /// Creates new filter options from command line arguments
    pub fn new(cli: &crate::cli::IndexArgs) -> Self {
        Self {
            len: cli.len,
            quality: cli.qual,
        }
    }
}

/// Determines if a read should be kept based on its length and quality
pub fn should_keep(read: &SequenceRecord, opts: &FilterOpts) -> bool {
    let quality_good = match read.phred_quality_avg() {
        Some(v) => opts.quality.contains(v),
        None => true, // if there is no quality, we do NOT want to remove
    };
    let len_good = opts.len.contains(read.num_bases() as f32);

    quality_good && len_good
}
