use crate::io::reads::record::QualityCompute;
use crate::utils;
use crate::{cli::get_version_label, io::index::storage::FileIndexPath};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Default)]
pub struct IndexMetadata {
    pub nailpolish_version: String,
    pub file_path: FileIndexPath,
    pub index_date: String,
    pub elapsed: f64,
    pub gb: f64,
    pub normal_reads: usize,
    pub invalid_reads: usize,
    pub filtered_reads: usize,
    pub total_reads: usize,
    pub avg_qual: f32,
    pub avg_len: f32,
}

impl IndexMetadata {
    pub fn add_read_metadata(
        &mut self,
        key: &super::DuplicateGroupKey,
        rec: needletail::parser::SequenceRecord,
    ) {
        // update metadata
        self.filtered_reads += key.is_filtered() as usize;
        self.normal_reads += key.is_normal() as usize;
        self.invalid_reads += key.is_invalid() as usize;
        self.total_reads += 1;

        self.avg_qual = utils::running_avg(
            self.avg_qual,
            rec.phred_quality_avg().unwrap_or(0.0),
            self.total_reads,
        );
        self.avg_len = utils::running_avg(self.avg_len, rec.num_bases() as f32, self.total_reads);
    }

    pub fn add_general_metadata(&mut self, file: FileIndexPath) {
        self.nailpolish_version = get_version_label();
        self.index_date = format!("{:?}", chrono::offset::Local::now());
        self.file_path = file;
    }

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
