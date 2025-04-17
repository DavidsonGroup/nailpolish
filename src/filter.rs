use crate::cli::ArgInterval;
use crate::io::InMemoryRecord;
use needletail::parser::SequenceRecord;
use std::iter::Map;
use std::slice::Iter;

pub struct FilterOpts {
    pub len: ArgInterval,
    pub quality: ArgInterval,
}

pub fn filter(read: &SequenceRecord, opts: &FilterOpts) -> bool {
    opts.len.contains(read.num_bases() as f64) && opts.quality.contains(read.phred_quality_avg())
}

pub(crate) trait QualityCompute {
    fn phred_quality(&self) -> Map<Iter<u8>, fn(&u8) -> u32>;

    fn phred_quality_avg(&self) -> f64;
}

impl QualityCompute for SequenceRecord<'_> {
    /// Returns the PHRED quality scores of the record as a byte slice.
    fn phred_quality(&self) -> Map<Iter<u8>, fn(&u8) -> u32> {
        // we transform the quality to a PHRED score (ASCII ! to I)
        // https://en.wikipedia.org/wiki/Phred_quality_score
        self.qual()?.into_iter().map(|&x| (x as u32) - 33u32)
    }

    /// Returns the average PHRED quality score of the record
    fn phred_quality_avg(&self) -> f64 {
        let qual = (self.phred_quality_total() as f64) / (self.len() as f64);
        // round to 2dp
        const ROUND_PRECISION: f64 = 100.0;
        (qual * ROUND_PRECISION).round() / ROUND_PRECISION
    }
}
