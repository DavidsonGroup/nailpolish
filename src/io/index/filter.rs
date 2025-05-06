use crate::{cli::interval::ArgInterval, io::reads::record::QualityCompute};
use needletail::parser::SequenceRecord;

pub struct FilterOpts {
    pub len: ArgInterval,
    pub quality: ArgInterval,
}

pub fn should_keep(read: &SequenceRecord, opts: &FilterOpts) -> bool {
    let quality_good = match read.phred_quality_avg() {
        Some(v) => opts.quality.contains(v),
        None => true, // if there is no quality, we do NOT want to remove
    };
    let len_good = opts.len.contains(read.num_bases() as f64);

    quality_good && len_good
}
