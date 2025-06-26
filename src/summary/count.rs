// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::collections::BTreeMap;

use crate::{
    io::index::{ArchivedIndex, DuplicateGroupLocation},
    utils,
};

#[derive(Debug, serde::Serialize)]
pub struct RowData {
    pub count: usize,
    pub avg_len: f32,
    pub avg_qual: f32,
}

impl RowData {
    pub fn new() -> Self {
        Self {
            count: 0,
            avg_len: 0.0,
            avg_qual: 0.0,
        }
    }

    pub fn add_group(&mut self, group: &DuplicateGroupLocation) {
        self.count += 1;

        let len = utils::complete_avg(group.reads.iter().map(|v| {
            let res: u32 = v.seq_len;
            res as f32
        }));

        let qual = utils::complete_avg(group.reads.iter().map(|v| v.qual));

        self.avg_len = utils::running_avg(self.avg_len, len, self.count);
        self.avg_qual = utils::running_avg(self.avg_qual, qual, self.count);
    }
}

pub fn summarize_index(index: &ArchivedIndex) -> IndexStatistics {
    debug!("Creating count index...");
    let mut map = BTreeMap::new();

    for group in index.groups() {
        let count = group.reads.len();

        let entry = map.entry(count).or_insert(RowData::new());
        entry.add_group(&group);
    }

    debug!("Count index created: {:?}", map);

    let metadata = index.metadata();

    IndexStatistics {
        nailpolish_version: metadata.nailpolish_version.clone(),
        file_path: metadata.file_path.fastq().display().to_string(),
        gb: metadata.gb as f32,
        index_date: metadata.index_date.clone(),
        read_count: metadata.total_reads,
        unfiltered_read_count: metadata.normal_reads,
        filtered_read_count: metadata.filtered_reads,
        avg_qual: metadata.avg_qual,
        avg_len: metadata.avg_len,
        stats: map,
    }
}

/// Statistics generated from analyzing a FASTQ index
#[derive(serde::Serialize)]
pub struct IndexStatistics {
    /// Version of nailpolish used to generate the index
    pub nailpolish_version: String,
    /// Path to the indexed FASTQ file
    pub file_path: String,
    /// Size of the file in gigabytes
    pub gb: f32,
    /// Date when the index was created
    pub index_date: String,
    /// Total number of reads
    pub read_count: usize,
    /// Number of reads before filtering
    pub unfiltered_read_count: usize,
    /// Number of reads after filtering
    pub filtered_read_count: usize,
    /// Average read quality score
    pub avg_qual: f32,
    /// Average read length
    pub avg_len: f32,
    /// Statistics grouped by number of reads
    pub stats: BTreeMap<usize, RowData>,
}
