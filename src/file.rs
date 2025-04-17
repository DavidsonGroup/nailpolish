use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Default)]
pub struct ReadFileMetadata {
    pub nailpolish_version: String,
    pub file_path: String,
    pub index_date: String,
    pub elapsed: f64,
    pub gb: f64,
    pub matched_read_count: usize,
    pub unmatched_read_count: usize,
    pub read_count: usize,
    pub total_qual: f64,
    pub total_len: f64,
    pub filtered_reads: usize,
}
