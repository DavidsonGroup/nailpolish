use crate::{cli::interval::ArgInterval, io::reads::record::QualityCompute};
use needletail::parser::SequenceRecord;

pub struct FilterOpts {
    pub len: ArgInterval,
    pub quality: ArgInterval,
}

impl FilterOpts {
    pub fn new(cli: &crate::cli::IndexArgs) -> Self {
        Self {
            len: cli.len.clone(),
            quality: cli.qual.clone(),
        }
    }
}

pub fn should_keep(read: &SequenceRecord, opts: &FilterOpts) -> bool {
    let quality_good = match read.phred_quality_avg() {
        Some(v) => opts.quality.contains(v),
        None => true, // if there is no quality, we do NOT want to remove
    };
    let len_good = opts.len.contains(read.num_bases() as f32);

    quality_good && len_good
}
