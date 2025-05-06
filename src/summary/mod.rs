use anyhow::Result;

pub mod statistics;

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
pub fn summarize(index: &str, output: &str) -> Result<()> {
    info!("Summarising index at {index}");
    todo!();
}
