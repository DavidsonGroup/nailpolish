// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

/// Metadata and statistics tracking for FASTQ index generation
use rkyv::{Archive, Deserialize, Serialize};

use crate::{
    cli::get_version_label,
    io::{index::storage::FileIndexPath, reads::record::QualityCompute},
    utils,
};

/// Stores metadata about an index, including read statistics and file information
#[derive(Archive, Serialize, Deserialize, Default)]
pub struct IndexMetadata {
    /// Version of nailpolish used to create the index
    pub nailpolish_version: String,
    /// Path to the indexed FASTQ file
    pub file_path: FileIndexPath,
    /// Timestamp when index was created
    pub index_date: String,
    /// Time taken to create index in seconds
    pub elapsed: f64,
    /// Size of indexed file in gigabytes
    pub gb: f64,
    /// Number of valid reads
    pub normal_reads: usize,
    /// Number of reads filtered by quality/length
    pub filtered_reads: usize,
    /// Total number of reads processed
    pub total_reads: usize,
    /// Average quality score across all reads
    pub avg_qual: f32,
    /// Average read length
    pub avg_len: f32,
}

impl IndexMetadata {
    /// Updates statistics for a single read, including counts and running averages
    pub fn add_read_metadata(&mut self, rec: needletail::parser::SequenceRecord) {
        // update metadata
        // self.filtered_reads += key.is_filtered() as usize;
        // self.normal_reads += key.is_normal() as usize;
        self.total_reads += 1;

        self.avg_qual = utils::running_avg(
            self.avg_qual,
            rec.phred_quality_avg().unwrap_or(0.0),
            self.total_reads,
        );
        self.avg_len = utils::running_avg(self.avg_len, rec.num_bases() as f32, self.total_reads);
    }

    /// Sets general index information like version and timestamp
    pub fn add_general_metadata(&mut self, file: FileIndexPath) {
        self.nailpolish_version = get_version_label();
        self.index_date = format!("{:?}", chrono::offset::Local::now());
        self.file_path = file;
    }

    /// Logs a summary of read processing statistics
    pub fn report_read_counts(&self) {
        info!(
            indoc::indoc! {"
                Statistics:
                  {} reads in total
                  {} valid
                  {} filtered out
                completed in {:.1}s runtime"
            },
            self.total_reads, self.normal_reads, self.filtered_reads, self.elapsed,
        )
    }
}
