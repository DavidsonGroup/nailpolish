// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::fmt::Write as StrWrite;
use std::io::Write as IoWrite;

use anyhow::Result;

const REPORT_INTERVAL: usize = 10000;

// CN: writer-struct; encapsulates progress tracking and output writing for consensus operations
pub struct OutputWriter<W: IoWrite> {
    writer: W,
    // Progress tracking
    processed_reads: usize,
    processed_clusters: usize,
    processed_duplicate_groups: usize,
    max_clusters: usize,
    filtered_reads: usize,

    // Configuration
    total_num_reads: usize,
    no_clustering: bool,
}

pub struct OutputMetadata {
    pub num_reads: usize,
    pub clusters: usize,
    pub num_filtered_reads: usize,
    pub is_duplicate: bool,
}

impl<W: IoWrite> OutputWriter<W> {
    pub fn new(writer: W, total_num_reads: usize, no_clustering: bool) -> Self {
        Self {
            writer,
            processed_reads: 0,
            processed_clusters: 0,
            processed_duplicate_groups: 0,
            max_clusters: 0,
            filtered_reads: 0,
            total_num_reads,
            no_clustering,
        }
    }

    pub fn write_read(&mut self, content: &str, metadata: &OutputMetadata) -> Result<()> {
        // Update progress counters
        self.processed_reads += metadata.num_reads;
        self.filtered_reads += metadata.num_filtered_reads;

        if metadata.is_duplicate {
            self.processed_clusters += metadata.clusters;
        }
        self.processed_duplicate_groups += metadata.is_duplicate as usize;
        self.max_clusters = std::cmp::max(self.max_clusters, metadata.clusters);

        // Report progress if interval reached
        if self.processed_reads % REPORT_INTERVAL == 0 {
            self.report_progress();
        }

        // Write content and flush
        write!(self.writer, "{}", content)?;

        Ok(())
    }

    fn report_progress(&self) {
        if self.no_clustering {
            info!(
                "proc: {} / {} reads (filtered: {})",
                self.processed_reads, self.total_num_reads, self.filtered_reads
            );
        } else {
            let cluster_ratio = if self.processed_duplicate_groups > 0 {
                self.processed_clusters as f64 / self.processed_duplicate_groups as f64
            } else {
                0.0
            };

            info!("proc: {} / {} reads\t(clusters per duplicate group: avg {:.2}, max {}; filtered: {})", 
                self.processed_reads, self.total_num_reads, cluster_ratio, self.max_clusters, self.filtered_reads);
        }
    }

    pub fn finalize(&mut self) -> Result<()> {
        // Final progress report
        self.report_progress();

        info!("Complete\ninput: {} reads\nduplicate groups: {} groups\ntotal clusters: {}\nfiltered reads: {}", 
            self.total_num_reads, self.processed_duplicate_groups, self.processed_clusters, self.filtered_reads);

        self.writer.flush()?;

        Ok(())
    }
}
