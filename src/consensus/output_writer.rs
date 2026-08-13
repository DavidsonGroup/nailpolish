// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::io::Write as IoWrite;

use anyhow::Result;

use crate::utils::fmt_count;

/// Encapsulates progress tracking and output writing for consensus operations
/// Note: any (usize, usize) tuple represents (group count, read count)
pub struct OutputWriter<W: IoWrite> {
    writer: W,

    // Progress tracking
    pre_fdd_singleton_groups: usize,
    pre_fdd_duplicate_groups: (usize, usize),
    post_fdd_singleton_groups: usize,
    post_fdd_duplicate_groups: (usize, usize),

    /// Groups dropped by filtering. Counted separately because they contribute
    /// no clusters, and so would otherwise drag the reported average below 1.
    filtered_reads: usize,

    // Configuration
    last_reported_reads: Option<usize>, // store last outputted read count to avoid duplicate progress lines
    num_reads: usize,
    total_num_reads: usize,
    no_clustering: bool,
    count_w: usize, // width of the count columns, sized for the largest value they can hold.
}

pub struct GroupConsensusMetadata {
    // group information
    pub is_duplicate_group: bool,
    pub post_fdd_singleton_groups: usize,
    pub post_fdd_duplicate_groups: (usize, usize),
    // read counts
    pub num_reads: usize,
    pub filtered_reads: usize,
}

impl<W: IoWrite> OutputWriter<W> {
    pub fn new(writer: W, total_num_reads: usize, no_clustering: bool) -> Self {
        // we use the .max(9) bound as the largest header value is
        // the word "filtered" / "duplicate", which is 9 chars long
        let count_w = fmt_count(total_num_reads).len().max(9);

        Self {
            writer,
            pre_fdd_singleton_groups: 0,
            pre_fdd_duplicate_groups: (0, 0),
            post_fdd_singleton_groups: 0,
            post_fdd_duplicate_groups: (0, 0),
            filtered_reads: 0,
            last_reported_reads: None,
            num_reads: 0,
            total_num_reads,
            no_clustering,
            count_w,
        }
    }

    pub fn write_read(&mut self, content: &str, metadata: &GroupConsensusMetadata) -> Result<()> {
        // Update progress counters
        if metadata.is_duplicate_group {
            self.pre_fdd_duplicate_groups = (
                self.pre_fdd_duplicate_groups.0 + 1,
                self.pre_fdd_duplicate_groups.1 + metadata.num_reads,
            );
        } else {
            self.pre_fdd_singleton_groups += 1;
        }

        self.post_fdd_duplicate_groups = (
            self.post_fdd_duplicate_groups.0 + metadata.post_fdd_duplicate_groups.0,
            self.post_fdd_duplicate_groups.1 + metadata.post_fdd_duplicate_groups.1,
        );
        self.post_fdd_singleton_groups += metadata.post_fdd_singleton_groups;

        self.num_reads += metadata.num_reads;
        self.filtered_reads += metadata.filtered_reads;

        // Write content
        write!(self.writer, "{}", content)?;

        Ok(())
    }

    /// Announces the run and prints the column headings for the progress lines.
    /// Call once, before the first [`Self::report_progress`].
    ///
    /// Totals and labels live here rather than on every progress line, so each
    /// line can be bare numbers.
    #[rustfmt::skip]
    pub fn report_header(&self, total_groups: usize) {
        info!(
            "Calling consensus on {} reads across {} groups",
            fmt_count(self.num_reads),
            fmt_count(total_groups)
        );
        info!("");

        let count_w = self.count_w;

        // this code produces a header for the progress lines, like this:
        //
        //   [14:34:57]                     ┌──── groups ─────┐   ┌─ false duplicates ─┐
        //   [14:34:57]    reads        %     single        dup   detections   avg/group   filtered
        //   [14:35:01]   14,141   100.0%     10,855      1,308          150        0.11          1
        //

        // the brackets span the group and detection columns, so those columns can
        // be labelled with the bare statistic
        let span = 2 * count_w + 3; // single + gutter + dup
        let pad = span - 10 - "groups".len();
        let groups_header = format!(
            "┌{} input groups {}┐",
            "─".repeat(pad / 2),
            "─".repeat(pad - pad / 2)
        );

        if self.no_clustering {
            info!("{:count_w$}            {groups_header}", "");
            info!("{:>count_w$}   {:>6}   {:>count_w$}   {:>count_w$}   {:>count_w$}",
                  "reads", "%", "singleton", "duplicate", "filtered");
        } else {
            info!("{:count_w$}            {groups_header}   ┌─ false duplicates ─┐", "");
            info!("{:>count_w$}   {:>6}   {:>count_w$}   {:>count_w$}   {:>10}   {:>9}   {:>count_w$}",
                  "reads", "%", "singleton", "duplicate", "detections", "avg/group", "filtered");
        }
    }

    /// Emits one progress line. Column widths must match [`Self::report_header`].
    pub fn report_progress(&mut self) {
        self.last_reported_reads = Some(self.num_reads);

        let percent = if self.num_reads > 0 {
            self.num_reads as f64 / self.total_num_reads as f64 * 100.0
        } else {
            0.0
        };

        let count_w = self.count_w;
        let reads = fmt_count(self.num_reads);
        let filtered = fmt_count(self.filtered_reads);

        // filtered groups yield no molecules, so they are excluded from the
        // duplicate count rather than dragging the rate down
        let sin_groups = fmt_count(self.pre_fdd_singleton_groups);
        let dup_groups = fmt_count(self.pre_fdd_duplicate_groups.0);

        if self.no_clustering {
            info!(
                "{reads:>count_w$}   {percent:>5.1}%   {sin_groups:>count_w$}   {dup_groups:>count_w$}   {filtered:>count_w$}"
            );
        } else {
            // how many groups post-FDD come from pre-FDD duplicate groups?
            let fdd_split_groups = self.post_fdd_duplicate_groups.0
                // any 'extra' singletons must have come from duplicate groups being split
                + (self
                    .post_fdd_singleton_groups
                    .saturating_sub(self.pre_fdd_singleton_groups));

            // every molecule beyond the first in a group is a false duplicate
            let detections = fdd_split_groups - self.pre_fdd_duplicate_groups.0;

            let avg = if self.pre_fdd_duplicate_groups.0 > 0 {
                detections as f64 / self.pre_fdd_duplicate_groups.0 as f64
            } else {
                0.0
            };

            let detections = fmt_count(detections);

            info!(
                "{reads:>count_w$}   {percent:>5.1}%   {sin_groups:>count_w$}   {dup_groups:>count_w$}   {detections:>10}   {avg:>9.2}   {filtered:>count_w$}"
            );
        }
    }

    #[rustfmt::skip]
    pub fn finalize(&mut self) -> Result<()> {
        // a report may already have fired on the last chunk, in which case this
        // would just repeat it
        if self.last_reported_reads != Some(self.num_reads) {
            self.report_progress();
        }
        info!("");

        if self.no_clustering {
            info!("Complete. Final counts:");
        } else {
            info!("Complete. Final counts, after false-duplicate detection:")
        }

        info!("  input reads          {}", self.total_num_reads);
        info!("  singleton groups     {}", self.post_fdd_singleton_groups);
        info!("  duplicate groups     {}   ({} reads)", self.post_fdd_duplicate_groups.0, self.post_fdd_duplicate_groups.1);
        info!("  filtered reads       {}", self.filtered_reads);

        self.writer.flush()?;

        Ok(())
    }
}
