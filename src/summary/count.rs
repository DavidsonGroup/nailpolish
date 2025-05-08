use std::collections::BTreeMap;

use crate::{
    io::index::{ArchivedDuplicateGroup, ArchivedIndex},
    utils,
};

#[derive(Debug)]
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

pub fn count_index(index: &ArchivedIndex) -> BTreeMap<usize, RowData> {
    debug!("Creating count index...");
    let mut map = BTreeMap::new();

    for group in index.groups() {
        let count = group.reads.len();

        let entry = map.entry(count).or_insert(RowData::new());
        entry.add_group(&group);
    }

    debug!("Count index created: {:?}", map);
    map
}
