use std::fs::File;
use std::io::BufWriter;

use crate::io::index::{ArchivedDuplicateGroupKey, FileIndexPath, IndexReader};

use anyhow::{Context, Result};

pub fn extract(args: &crate::cli::ExtractArgs) -> anyhow::Result<()> {
    let paths = FileIndexPath::new(&args.input);
    let index_rdr = IndexReader::new(&paths)?;

    info!(
        "Grouping reads from {} using index {} → {}",
        paths.fastq().display(),
        paths.index().display(),
        args.output
            .as_ref()
            .map_or("stdout".to_string(), |v| v.display().to_string())
    );

    let index = index_rdr.load()?;

    let allowed_ids = if let Some(id_str) = &args.id {
        id_str
            .split(',')
            .map(|s| {
                let trimmed = s.trim();
                trimmed.parse::<usize>()
            })
            .collect::<Result<Vec<usize>, std::num::ParseIntError>>()
            .context("Invalid ID string")?
    } else if let Some(key) = &args.key {
        let re = regex::Regex::new(key)?;

        index
            .groups()
            .filter_map(|group| {
                if let ArchivedDuplicateGroupKey::Normal(k) = group.key {
                    if re.is_match(&k.0) {
                        return Some(group.id);
                    }
                }

                None
            })
            .collect()
    } else if let Some(group_size) = &args.group_size {
        index
            .groups()
            .filter_map(|group| {
                if group.reads.len() == *group_size {
                    Some(group.id)
                } else {
                    None
                }
            })
            .collect()
    } else {
        anyhow::bail!("No key or ID is passed")
    };

    let allowed_groups = allowed_ids.iter().map(|id| index.get_by_id(*id));

    let mut accessor = index
        .get_read_accessor(&paths)
        .context("Failed to create read accessor")?;

    let mut writer = crate::utils::get_writer(args.output.as_deref())?;

    for group in allowed_groups {
        let group = group.context("Group does not exist")?;
        let reads = accessor.fetch_reads_archived(group.reads)?;

        writer.write_all(&reads)?;
    }

    writer.flush()?;
    info!("Finished");

    Ok(())
}
