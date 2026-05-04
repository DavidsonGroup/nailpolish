// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

//! Provides functionality for generating HTML summaries of index files.
//! Uses a template-based approach with handlebars for rendering.

use std::io::Write;

use anyhow::Result;

use crate::{io::index::IndexReader, summary::count::IndexStatistics};

use count::summarize_index;

// Internal module for counting statistics
mod count;

// Load the HTML template at compile time
const TEMPLATE_HTML: &str = include_str!("summary_template.html");

/// Generates an HTML summary report for the given index file.
pub fn summarize(args: &crate::cli::SummaryArgs) -> Result<()> {
    let paths = crate::io::index::FileIndexPath::new(&args.input);

    // try to open file
    let summary_file = args
        .output
        .clone()
        .unwrap_or_else(|| paths.fastq().with_extension("summary.html"));

    let index = IndexReader::new(&paths)?;

    // report action
    info!(
        "Summarising {} → {}",
        paths.fastq().display(),
        summary_file.display()
    );

    let index = index.load()?;
    let mut file = std::fs::File::create_new(&summary_file)?;

    let stats = summarize_index(index);
    let mut json = serde_json::json!(stats);

    // we must convert this to a string so it imports correctly
    json["stats"] = serde_json::json!(serde_json::to_string(&stats.stats)?);

    debug!("serde_json: {json:?}");

    // Use the handlebars crate to render the template with the stats
    let mut handlebars = handlebars::Handlebars::new();
    handlebars.set_strict_mode(true);

    // Render the template
    handlebars.register_template_string("t_summary", TEMPLATE_HTML)?;
    let rendered_html = handlebars.render("t_summary", &json)?;
    write!(file, "{}", rendered_html)?;

    // report result
    info!("Summary written to {}. In brief:", summary_file.display());
    print_stats_table(&stats);

    Ok(())
}

#[rustfmt::skip]
fn print_stats_table(stats: &IndexStatistics) {
        info!("──────────────────────────┬───────────────────────────────────────────────────");
        info!("  Nailpolish version      │ {}", stats.nailpolish_version);
        info!("  File path               │ {}", stats.file_path);
        info!("  Dataset size            │ {} GB", stats.gb);
        info!("  Index date              │ {}", stats.index_date);
        info!("  Total read count        │ {}", stats.read_count);
        info!("  Reads with barcodes     │ {}", stats.unfiltered_read_count);
        info!("  Reads without barcodes  │ {}", stats.filtered_read_count);
        info!("  Average quality         │ {}", stats.avg_qual);
        info!("  Average length          │ {}", stats.avg_len);
        info!("──────────────────────────┴───────────────────────────────────────────────────");
}
