/// Index construction and read identifier parsing
use super::filter::should_keep;
use super::storage::FileIndexPath;
use super::{DuplicateGroupKey, Index, ReadLocation, ReadLocationTrait, RecordIdentifier};
use crate::io::reads::QualityCompute;

use anyhow::{bail, Result};
use humansize::{format_size, FormatSizeOptions};
use needletail::parser::SequenceRecord;
use needletail::FastxReader;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek};
use std::path::Path;
use thiserror::Error;

/// Constructs an index file for a FASTQ file, extracting barcodes and UMIs
/// from read headers using either a regex pattern or a cluster file
pub fn construct_index(cli: &crate::cli::IndexArgs) -> Result<()> {
    let path = FileIndexPath::new(&cli.input);

    if path.index().exists() {
        if cli.overwrite {
            info!("Index already exists, but the `--overwrite` flag was passed, so overwriting")
        } else {
            bail!(indoc::formatdoc! { "
            Index file `{}` already exists

            suggestion: run nailpolish with '--overwrite' to disable file check, or delete existing index"
            , path.index().display() })
        }
    }

    // create the .fastq reader
    let f = File::open(path.fastq()).expect("File could not be opened");
    let metadata = f.metadata()?;

    let total_bytes = metadata.len();

    let mut reader = BufReader::new(f);

    let mut index = Index::new(path);

    let filters = super::filter::FilterOpts::new(cli);

    let size_formatter = FormatSizeOptions::from(humansize::BINARY)
        .decimal_places(1)
        .decimal_zeroes(1)
        .units(humansize::Kilo::Decimal)
        .space_after_value(false);

    const REPORT_INTERVAL: u64 = 2 * 1024 * 1024 * 1024; // 2GB
    let mut last_report = 0;

    // callback function to process each read that comes in & add to the index
    let callback = |loc: ReadLocation, key: DuplicateGroupKey, seq: SequenceRecord| {
        let pos = loc.pos();

        // should we report our current progress?
        if pos - last_report > REPORT_INTERVAL {
            last_report = pos;

            info!(
                "proc: {} / {}",
                format_size(pos, size_formatter),
                format_size(total_bytes, size_formatter)
            );
        }

        let key = if !should_keep(&seq, &filters) {
            DuplicateGroupKey::Filtered(pos)
        } else {
            key
        };

        index.add_read(key, loc, seq);
        Ok(())
    };

    if let Some(cluster_file) = &cli.clusters {
        iter_lines_with_cluster_file(&mut reader, cluster_file, callback)?;
    } else {
        let re = match &cli.barcode_regex {
            Some(v) => {
                info!("Using barcode format {v}");
                regex::Regex::new(v)
            }
            None => cli.preset.to_regex(),
        }?;

        iter_lines_with_regex(&mut reader, &re, callback)?
    }

    info!(
        "proc: {} / {}",
        format_size(total_bytes, size_formatter),
        format_size(total_bytes, size_formatter)
    );

    index.mark_indexation_complete(reader.stream_position()? as f64 / (1024.0 * 1024.0))?;
    index.metadata().report_read_counts();

    index.write()?;

    Ok(())
}

/// Process FASTQ reads using a regex to extract identifiers from headers
fn iter_lines_with_regex<F>(
    reader: &mut BufReader<File>,
    re: &regex::Regex,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(ReadLocation, DuplicateGroupKey, SequenceRecord) -> Result<()>,
{
    // expected_len is used to ensure that every read has the same format
    let mut expected_len: Option<usize> = None;

    let mut fastq_reader = needletail::parser::FastqReader::new(reader);

    while let Some(rec) = fastq_reader.next() {
        let rec = rec.expect("Invalid record");

        let read_location = ReadLocation {
            _pos: rec.position().byte(),
            _byte_len: (rec.all().len() as u32) + 1,
            seq_len: rec.num_bases() as u32,
            qual: rec.phred_quality_avg().unwrap_or_default(),
        };
        let header = std::str::from_utf8(rec.id())?;

        let (len, id) = extract_header_id(header, re, read_location.pos())?;
        
        // check # of barcode groups is the same
        let expected_len = *expected_len.get_or_insert(len);
        if expected_len != len {
            bail!(IndexGenerationErr::DifferentMatchCounts {
                header: header.to_string(),
                re: re.clone(),
                pos: read_location.pos(),
                count: len,
                expected: expected_len
            })
        }

        let barcode_location = DuplicateGroupKey::Normal(id);

        callback(read_location, barcode_location, rec)?;
    }
    Ok(())
}

fn iter_lines_with_cluster_file<F>(
    reader: &mut BufReader<File>,
    cluster_file: &Path,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(ReadLocation, DuplicateGroupKey, SequenceRecord) -> Result<()>,
{
    // Read cluster file line by line
    info!("Reading identifiers from file {}", cluster_file.display());

    let cluster_f = File::open(cluster_file)?;
    let cluster_reader = BufReader::new(cluster_f);
    let mut cluster_map = HashMap::new();

    for line in cluster_reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split(';').collect();
        if parts.len() != 2 {
            bail!(IndexGenerationErr::InvalidClusterRow {
                row: line.to_string()
            });
        }

        let read_id = parts[0].to_string();
        let identifier = RecordIdentifier::from_recs(&[parts[1]]);
        cluster_map.insert(read_id, identifier);
    }

    info!(
        "Finished reading clusters. Found {} cluster mappings",
        cluster_map.len()
    );

    let mut fastq_reader = needletail::parser::FastqReader::new(reader);

    while let Some(rec) = fastq_reader.next() {
        let rec = rec.expect("Invalid record");

        let read_location = ReadLocation {
            _pos: rec.position().byte(),
            _byte_len: (rec.all().len() as u32) + 1,
            seq_len: rec.num_bases() as u32,
            qual: rec.phred_quality_avg().unwrap_or_default(),
        };

        let header = std::str::from_utf8(rec.id())?;

        let key = match cluster_map.get(header) {
            Some(identifier) => DuplicateGroupKey::Normal(identifier.clone()),
            None => {
                bail!(IndexGenerationErr::RowNotInClusters {
                    header: header.to_string()
                });
            }
        };

        callback(read_location, key, rec)?;
    }

    Ok(())
}

/// Extract identifier components from a read header using a regex pattern
/// Returns the number of captures and the constructed identifier
fn extract_header_id(header: &str, re: &Regex, pos: u64) -> Result<(usize, RecordIdentifier)> {
    let Some(captures) = re.captures(header) else {
        bail!(IndexGenerationErr::NoMatch {
            header: String::from(header.trim()),
            re: re.clone(),
            pos
        });
    };

    let captures = captures
        .iter()
        .skip(1)
        .flatten()
        .map(|m| m.as_str())
        .collect::<Vec<_>>();

    Ok((captures.len(), RecordIdentifier::from_recs(&captures)))
}

/// Errors that can occur during index generation
#[derive(Error, Debug)]
enum IndexGenerationErr {
    #[error(
        "no matches produced:
position {pos}
    `{header}`
with capture group
    {re:?}
suggestion: inspect the read using `tail -c +{pos} <fastq> | head -n 5`"
    )]
    NoMatch { header: String, re: Regex, pos: u64 },

    #[error(
        "inconsistent identifier count:
position {pos}
    `{header}`
has {count} matches, whereas {expected} matches were expected
using capture group
    {re:?}"
    )]
    DifferentMatchCounts {
        header: String,
        re: Regex,
        pos: u64,
        count: usize,
        expected: usize,
    },

    #[error(
        "invalid cluster row: should be of the format
  `READ_ID;BC;UMI`
or
  `READ_ID;BC`, but instead got
{row}"
    )]
    InvalidClusterRow { row: String },

    #[error("Row {header} of input file not present in cluster file")]
    RowNotInClusters { header: String },
}
