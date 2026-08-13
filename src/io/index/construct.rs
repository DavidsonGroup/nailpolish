// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

/// Index construction and read identifier parsing
use std::{collections::HashMap, fs::File, io::Seek};

use anyhow::{bail, Result};
use humansize::{format_size, FormatSizeOptions};
use itertools::Itertools;
use needletail::{parser::SequenceRecord, FastxReader};
use regex::Regex;
use thiserror::Error;

use super::{storage::FileIndexPath, Index, ReadLocation, RecordIdentifier};
use crate::io::reads::{QualityCompute, SequentialReader};

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

    let mut reader = SequentialReader::from_path(path.fastq())?;
    let mut index = Index::new(path);

    let size_formatter = FormatSizeOptions::from(humansize::BINARY)
        .decimal_places(1)
        .decimal_zeroes(1)
        .units(humansize::Kilo::Decimal)
        .space_after_value(false);

    const REPORT_INTERVAL: u64 = 2 * 1024 * 1024 * 1024; // 2GB
    let mut last_report = 0;

    // a formatted size is at widest `1023.9XB`; fixed so rows stay aligned even
    // as the unit changes partway through a file
    const BYTES_W: usize = 8;

    let report = |pos: u64| {
        let percent = if total_bytes > 0 {
            pos as f64 / total_bytes as f64 * 100.0
        } else {
            0.0
        };

        let bytes = format_size(pos, size_formatter);
        info!("{bytes:>BYTES_W$}   {percent:>5.1}%");
    };

    info!(
        "Indexing {} from {}",
        format_size(total_bytes, size_formatter),
        cli.input.display()
    );
    info!("");
    info!("{:>BYTES_W$}   {:>6}", "bytes", "%");

    let skipped = if let Some(cluster_file) = &cli.clusters {
        // Read cluster file line by line
        info!("Reading identifiers from file {}", cluster_file.display());

        // create cluster reader
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .delimiter(b';')
            .from_path(cluster_file)?;

        index.add_captures_from_csv_headers(rdr.headers()?);

        // callback function to process each read that comes in & add to the index
        let callback = |loc: ReadLocation, key: RecordIdentifier, seq: SequenceRecord| {
            let pos = loc.pos();

            if pos - last_report > REPORT_INTERVAL {
                last_report = pos;
                report(pos);
            }

            index.add_read(key, loc, seq);
            Ok(())
        };

        // iterate over reads and call fn callback()
        iter_lines_with_cluster_file(&mut reader, rdr, cli.skip_unmatched, callback)?
    } else {
        // Determine if manual regex or preset should be used
        let re = match &cli.barcode_regex {
            Some(v) => {
                info!("Using barcode format {v}");
                regex::Regex::new(v)
            }
            None => cli.preset.to_regex(),
        }?;

        index.add_captures_from_regex(&re);

        // callback function to process each read that comes in & add to the index
        let callback = |loc: ReadLocation, key: RecordIdentifier, seq: SequenceRecord| {
            let pos = loc.pos();

            if pos - last_report > REPORT_INTERVAL {
                last_report = pos;
                report(pos);
            }

            index.add_read(key, loc, seq);
            Ok(())
        };

        // iterate over reads and call fn callback()
        iter_lines_with_regex(&mut reader, &re, cli.skip_unmatched, callback)?
    };

    report(total_bytes);
    info!("");

    if skipped > 0 {
        warn!("{skipped} reads were skipped (not matched/found in cluster file)");
    }

    let final_position = reader.stream_position()? as f64 / (1024.0 * 1024.0 * 1024.0);

    // finalise the gzip index, if the source file is a .gzip file
    reader.finish_gzip_index()?;

    index.mark_indexation_complete(final_position, skipped)?;
    index.metadata().report_read_counts();

    index.write()?;

    Ok(())
}

/// Process FASTQ reads using a regex to extract identifiers from headers
fn iter_lines_with_regex<F>(
    reader: &mut SequentialReader,
    re: &regex::Regex,
    skip_unmatched: bool,
    mut callback: F,
) -> Result<usize>
where
    F: FnMut(ReadLocation, RecordIdentifier, SequenceRecord) -> Result<()>,
{
    // expected_len is used to ensure that every read has the same format
    let mut expected_len: Option<usize> = None;
    let mut skipped: usize = 0;

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

        // CN: skip-unmatched; None means no regex match; DifferentMatchCounts is never skipped
        let Some((len, id)) = extract_header_id(header, re)? else {
            if skip_unmatched {
                skipped += 1;
                continue;
            }
            bail!(IndexGenerationErr::NoMatch {
                header: header.trim().to_string(),
                re: re.clone(),
                pos: read_location.pos(),
            })
        };

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

        callback(read_location, id, rec)?;
    }
    Ok(skipped)
}

fn iter_lines_with_cluster_file<F, R>(
    reader: &mut SequentialReader,
    mut rdr: csv::Reader<R>,
    skip_unmatched: bool,
    mut callback: F,
) -> Result<usize>
where
    F: FnMut(ReadLocation, RecordIdentifier, SequenceRecord) -> Result<()>,
    R: std::io::Read,
{
    let mut cluster_map = HashMap::new();

    for result in rdr.records() {
        let record = result?;

        let read_id = record[0].to_string();
        let identifier = RecordIdentifier::from_recs(&record.iter().skip(1).collect_vec());

        cluster_map.insert(read_id, identifier);
    }

    info!(
        "Finished reading clusters. Found {} cluster mappings",
        cluster_map.len()
    );

    let mut fastq_reader = needletail::parser::FastqReader::new(reader);
    let mut skipped: usize = 0;

    while let Some(rec) = fastq_reader.next() {
        let rec = rec.expect("Invalid record");

        let read_location = ReadLocation {
            _pos: rec.position().byte(),
            _byte_len: (rec.all().len() as u32) + 1,
            seq_len: rec.num_bases() as u32,
            qual: rec.phred_quality_avg().unwrap_or_default(),
        };

        let header = std::str::from_utf8(rec.id())?;

        // CN: skip-unmatched; honour --skip-unmatched by counting rather than bailing
        let Some(id) = cluster_map.get(header) else {
            if skip_unmatched {
                skipped += 1;
                continue;
            }
            bail!(IndexGenerationErr::RowNotInClusters {
                header: header.to_string()
            })
        };
        let id = id.clone();

        callback(read_location, id, rec)?;
    }

    Ok(skipped)
}

/// Extract identifier components from a read header using a regex pattern.
/// Returns `Ok(None)` when the header doesn't match; the caller decides whether to skip or bail.
fn extract_header_id(header: &str, re: &Regex) -> Result<Option<(usize, RecordIdentifier)>> {
    let Some(captures) = re.captures(header) else {
        return Ok(None);
    };

    let captures = captures
        .iter()
        .skip(1)
        .flatten()
        .map(|m| m.as_str())
        .collect::<Vec<_>>();

    Ok(Some((
        captures.len(),
        RecordIdentifier::from_recs(&captures),
    )))
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

    #[error("Row {header} of input file not present in cluster file")]
    RowNotInClusters { header: String },
}
