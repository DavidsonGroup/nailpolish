use std::collections::BTreeMap;

use crate::{
    io::index::{ArchivedDuplicateGroup, ArchivedIndex},
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

    pub fn add_group(&mut self, group: &ArchivedDuplicateGroup) {
        self.count += 1;

        let len = utils::complete_avg(group.reads.iter().map(|v| {
            let res: u32 = v.seq_len.try_into().unwrap();
            res as f32
        }));

        let qual = utils::complete_avg(group.reads.iter().map(|v| v.qual.try_into().unwrap()));

        self.avg_len = utils::running_avg(self.avg_len, len, self.count);
        self.avg_qual = utils::running_avg(self.avg_qual, qual, self.count);
    }
}

pub fn count_index(index: &ArchivedIndex) -> IndexStatistics {
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

#[derive(serde::Serialize)]
pub struct IndexStatistics {
    pub nailpolish_version: String,
    pub file_path: String,
    pub gb: f32,
    pub index_date: String,
    pub read_count: usize,
    pub unfiltered_read_count: usize,
    pub filtered_read_count: usize,
    pub avg_qual: f32,
    pub avg_len: f32,
    pub stats: BTreeMap<usize, RowData>,
}
