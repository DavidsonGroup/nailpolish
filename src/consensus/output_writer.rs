// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::io::Write as IoWrite;

use anyhow::Result;

use crate::utils::fmt_count;

// CN: writer-struct; encapsulates progress tracking and output writing for consensus operations
pub struct OutputWriter<W: IoWrite> {
    writer: W,
    // Progress tracking
    processed_reads: usize,
    processed_clusters: usize,
    processed_duplicate_groups: usize,
    /// Groups dropped by filtering. Counted separately because they contribute
    /// no clusters, and so would otherwise drag the reported average below 1.
    filtered_groups: usize,
    max_clusters: usize,
    filtered_reads: usize,
    /// Read count at the last progress line, so the final one isn't repeated.
    last_reported_reads: Option<usize>,

    // Configuration
    total_num_reads: usize,
    no_clustering: bool,
    /// Width of the count columns, sized for the largest value they can hold.
    count_w: usize,
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
            filtered_groups: 0,
            max_clusters: 0,
            filtered_reads: 0,
            last_reported_reads: None,
            total_num_reads,
            no_clustering,
            count_w: fmt_count(total_num_reads).len().max("filtered".len()),
        }
    }

    pub fn write_read(&mut self, content: &str, metadata: &OutputMetadata) -> Result<()> {
        // Update progress counters
        self.processed_reads += metadata.num_reads;
        self.filtered_reads += metadata.num_filtered_reads;

        if metadata.is_duplicate {
            self.processed_clusters += metadata.clusters;
            // a duplicate group producing no clusters was filtered out
            self.filtered_groups += (metadata.clusters == 0) as usize;
        }
        self.processed_duplicate_groups += metadata.is_duplicate as usize;
        self.max_clusters = std::cmp::max(self.max_clusters, metadata.clusters);

        // Write content
        write!(self.writer, "{}", content)?;

        Ok(())
    }

    /// Announces the run and prints the column headings for the progress lines.
    /// Call once, before the first [`Self::report_progress`].
    ///
    /// Totals and labels live here rather than on every progress line, so each
    /// line can be bare numbers.
    pub fn report_header(&self, total_groups: usize) {
        info!(
            "Calling consensus on {} reads across {} groups",
            fmt_count(self.total_num_reads),
            fmt_count(total_groups)
        );
        info!("");

        let count_w = self.count_w;

        if self.no_clustering {
            info!(
                "{:>count_w$}   {:>6}   {:>count_w$}",
                "reads", "%", "filtered"
            );
        } else {
            // the bracket spans the avg and max columns, so they can be labelled
            // with the bare statistic
            info!("{:count_w$}            ┌────── per dup group ──────┐", "");
            info!(
                "{:>count_w$}   {:>6}   {:>13}   {:>13}   {:>count_w$}",
                "reads", "%", "avg molecules", "max molecules", "filtered"
            );
        }
    }

    /// Emits one progress line. Column widths must match [`Self::report_header`].
    pub fn report_progress(&mut self) {
        self.last_reported_reads = Some(self.processed_reads);

        let percent = if self.total_num_reads > 0 {
            self.processed_reads as f64 / self.total_num_reads as f64 * 100.0
        } else {
            0.0
        };

        let count_w = self.count_w;
        let reads = fmt_count(self.processed_reads);
        let filtered = fmt_count(self.filtered_reads);

        if self.no_clustering {
            info!("{reads:>count_w$}   {percent:>5.1}%   {filtered:>count_w$}");
        } else {
            // filtered groups produce no clusters, so they are excluded from the
            // denominator rather than dragging the average below 1
            let clustered_groups = self.processed_duplicate_groups - self.filtered_groups;
            let avg = if clustered_groups > 0 {
                self.processed_clusters as f64 / clustered_groups as f64
            } else {
                0.0
            };
            let max = fmt_count(self.max_clusters);

            info!(
                "{reads:>count_w$}   {percent:>5.1}%   {avg:>13.2}   {max:>13}   {filtered:>count_w$}"
            );
        }
    }

    pub fn finalize(&mut self) -> Result<()> {
        // a report may already have fired on the last chunk, in which case this
        // would just repeat it
        if self.last_reported_reads != Some(self.processed_reads) {
            self.report_progress();
        }
        info!("");

        let summary = [
            ("input reads", self.total_num_reads),
            ("duplicate groups", self.processed_duplicate_groups),
            ("total molecules", self.processed_clusters),
            ("filtered reads", self.filtered_reads),
        ];

        // labels left-aligned, values right-aligned, so both form columns
        let label_width = summary.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
        let value_width = fmt_count(self.total_num_reads).len();

        info!("Complete");
        for (label, value) in summary {
            info!(
                "  {label:<label_width$}   {:>value_width$}",
                fmt_count(value)
            );
        }

        self.writer.flush()?;

        Ok(())
    }
}
