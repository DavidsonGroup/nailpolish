// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::io::{Cursor, Write};

use anyhow::{Context, Result};
use needletail::{parser::FastqReader, FastxReader};

use crate::{
    cli::preset::PresetOutputFormats,
    io::index::{FileIndexPath, IndexReader},
};

pub fn extract(args: &crate::cli::ExtractArgs) -> anyhow::Result<()> {
    let paths = FileIndexPath::new(&args.input);
    let index_rdr = IndexReader::new(&paths)?;

    info!(
        "Extracting reads from {} using index {} → {}",
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
            .groups_by_index(&index.indices_by_default_order())
            .filter_map(|group| {
                if re.is_match(&group.key.0) {
                    return Some(group.id);
                }
                None
            })
            .collect()
    } else if let Some(group_size) = &args.group_size {
        index
            .groups_by_index(&index.indices_by_default_order())
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

    println!("Test");
    if args.format == PresetOutputFormats::Fastq {
        for group in allowed_groups {
            let group = group.context("Group does not exist")?;
            let reads_u8 = accessor.fetch_reads(&group.reads)?.concat();

            writer.write_all(&reads_u8)?;
        }
    } else if args.format == PresetOutputFormats::Fasta {
        for group in allowed_groups {
            let group = group.context("Group does not exist")?;
            let reads_u8 = accessor.fetch_reads(&group.reads)?.concat();

            let mut reader = FastqReader::new(Cursor::new(reads_u8));
            while let Some(read) = reader.next() {
                let read = read.context("Invalid read")?;

                writeln!(writer, ">{}", String::from_utf8_lossy(read.id()))?;
                writeln!(writer, "{}", String::from_utf8_lossy(&read.seq()))?;
            }
        }
    } else if args.format == PresetOutputFormats::Metadata {
        for group in allowed_groups {
            let group = group.unwrap();
            writeln!(writer, "id: {}, key: {:?}", group.id, group.key)?;
        }
    }

    writer.flush()?;
    info!("Finished");

    Ok(())
}
