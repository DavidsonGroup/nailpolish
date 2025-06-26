// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use needletail::parser::SequenceRecord;

pub trait QualityCompute {
    fn phred_quality(&self) -> Option<impl IntoIterator<Item = u32>>;

    fn phred_quality_avg(&self) -> Option<f32>;
}

const ROUND_PRECISION: f32 = 100.0;

impl QualityCompute for SequenceRecord<'_> {
    /// Returns the PHRED quality scores of the record as a byte slice.
    fn phred_quality(&self) -> Option<impl IntoIterator<Item = u32>> {
        // we transform the quality to a PHRED score (ASCII ! to I)
        // https://en.wikipedia.org/wiki/Phred_quality_score
        Some(self.qual()?.iter().map(|&x| (x as u32) - 33u32))
    }

    /// Returns the average PHRED quality score of the record
    fn phred_quality_avg(&self) -> Option<f32> {
        let qual =
            (self.phred_quality()?.into_iter().sum::<u32>() as f32) / (self.num_bases() as f32);
        // round to 2dp
        Some((qual * ROUND_PRECISION).round() / ROUND_PRECISION)
    }
}
