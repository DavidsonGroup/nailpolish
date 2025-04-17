use crate::{duplicates, r#mod};
use anyhow::{Context, Result};
use serde_json::json;

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
    let mut index = r#mod::IndexReader::from_path(index)?;
    let (_, statistics) = index.get_duplicates()?;
    let gb = index.metadata.gb;

    let mut data = serde_json::to_value(index.metadata).context("Could not serialize info")?;

    println!("{}", serde_json::to_string(&statistics)?);
    // round "gb" stat to 3dp
    data["gb"] = json!(format!("{:.3}", gb));
    data["stats"] = json!(serde_json::to_string(&statistics)?);

    println!(
        "{}",
        serde_json::to_string_pretty(&data).context("Should be serialisable")?
    );

    let file = std::fs::File::create(output)?;
    let reg = handlebars::Handlebars::new();
    reg.render_template_to_write(TEMPLATE_HTML, &data, file)?;

    Ok(())
}
