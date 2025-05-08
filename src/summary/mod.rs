use anyhow::Result;

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
    paths.check_indexed()?;

    let summary_file = args
        .output
        .clone()
        .unwrap_or_else(|| paths.fastq().with_extension("summary.html"));

    info!(
        "Summarising {} → {}",
        paths.fastq().display(),
        summary_file.display()
    );
    todo!();
}
