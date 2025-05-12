use anyhow::Result;
use count::count_index;
use std::io::Write;

use crate::io::index::IndexReader;
mod count;

// encode the template HTML file at compile time as a string literal
const TEMPLATE_HTML: &str = include_str!("summary_template.html");

/// Summarizes the index and writes the output in HTML format to a file.
///
/// # Arguments
///
/// * `index` - A string slice that holds the path to the index file.
/// * `output` - A string slice that holds the path to the output file.
///
/// # Returns
///
/// * `Result<()>` - Returns an `Ok(())` if successful, or an `anyhow::Error` if an error occurs.
pub fn summarize(args: &crate::cli::SummaryArgs) -> Result<()> {
    let paths = crate::io::index::FileIndexPath::new(&args.input);

    let summary_file = args
        .output
        .clone()
        .unwrap_or_else(|| paths.fastq().with_extension("summary.html"));

    let index = IndexReader::new(&paths)?;

    info!(
        "Summarising {} → {}",
        paths.fastq().display(),
        summary_file.display()
    );

    let index = index.load()?;
    let mut file = std::fs::File::create_new(&summary_file)?;

    let stats = count_index(index);
    let mut json = serde_json::json!(stats);
    json["stats"] = serde_json::json!(serde_json::to_string(&stats.stats)?);
    info!("Serde json: {json:?}");

    // Use the handlebars crate to render the template with the stats
    let mut handlebars = handlebars::Handlebars::new();
    handlebars.set_strict_mode(true);

    // Render the template
    handlebars.register_template_string("summary", TEMPLATE_HTML)?;
    let rendered_html = handlebars.render("summary", &json)?;

    write!(file, "{}", rendered_html)?;
    info!("Summary written to {}", summary_file.display());

    Ok(())
}
