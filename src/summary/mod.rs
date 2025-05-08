use anyhow::Result;
use count::count_index;

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

    let map = count_index(index);
    todo!();
}
