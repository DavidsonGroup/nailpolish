/// Index construction and read identifier parsing
use super::filter::should_keep;
use super::storage::FileIndexPath;
use super::{DuplicateGroupKey, Index, ReadLocation, ReadLocationTrait, RecordIdentifier};
use crate::io::reads::QualityCompute;

use anyhow::{bail, Result};
use needletail::parser::SequenceRecord;
use needletail::FastxReader;
use regex::Regex;
use std::fs::File;
use std::io::{BufReader, Seek};
use std::path::Path;
use thiserror::Error;

/// Constructs an index file for a FASTQ file, extracting barcodes and UMIs
/// from read headers using either a regex pattern or a cluster file
pub fn construct_index(cli: &crate::cli::IndexArgs) -> Result<()> {
    let path = FileIndexPath::new(&cli.file);

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
    let mut reader = BufReader::new(f);

    let mut index = Index::new(path);

    let filters = super::filter::FilterOpts::new(cli);

    // callback function to process each read that comes in & add to the index
    let callback = |loc: ReadLocation, key: DuplicateGroupKey, seq: SequenceRecord| {
        let key = if !key.is_invalid() && !should_keep(&seq, &filters) {
            DuplicateGroupKey::Filtered(loc.pos())
        } else {
            key
        };

        index.add_read(key, loc, seq);
        Ok(())
    };

    if let Some(loc) = &cli.clusters {
        todo!();
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

    index.mark_indexation_complete(reader.stream_position()? as f64 / (1024.0 * 1024.0))?;
    index.metadata().report_read_counts();

    index.write()?;

    info!("Complete");

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
            _pos: rec.position().byte() as usize,
            _byte_len: rec.all().len() + 1,
            seq_len: rec.num_bases(),
            qual: rec.phred_quality_avg().unwrap_or_default(),
        };
        let header = std::str::from_utf8(rec.id())?;

        let bc = extract_header_id(header, re, read_location.pos());
        let barcode_location = match bc {
            Ok((len, id)) => {
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

                DuplicateGroupKey::Normal(id)
            }
            Err(e) => DuplicateGroupKey::Invalid(read_location.pos()),
        };

        callback(read_location, barcode_location, rec)?;
    }
    Ok(())
}

fn iter_lines_with_cluster_file(
    reader: BufReader<File>,
    // wtr: &mut IndexWriter,
    cluster_file: &Path,
    skip_invalid_ids: bool,
) -> Result<()> {
    todo!();
    /*
    todo: filepath
        let mut cluster_rdr = csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(false)
            .from_path(filepath)?;

    // first, we will read the clusters file
    info!("Reading identifiers from clusters file...");

    let mut cluster_map = std::collections::HashMap::new();

    for result in clusters.records() {
        let record = result?;

        let read_id = record[0].to_string();
        let identifier = match record.len() {
            // in this case, there is just one identifier (no BC and UMI) so we read the first
            // column directly as the 'identifier'
            2 => record[1].to_string(),

            // in this case, there are two identifiers (i.e. BC and UMI) so we combine them to
            // produce an 'identifier'
            3 => format!("{}_{}", &record[1], &record[2]),

            // doesn't make sense
            _ => bail!(InvalidClusterRow {
                row: record.as_slice().to_string()
            }),
        };

        cluster_map.insert(read_id, identifier);
    }

    info!("Finished reading clusters. ");

    let mut fastq_reader = needletail::parser::FastqReader::new(reader);

    while let Some(rec) = fastq_reader.next() {
        let rec = rec.expect("Invalid record");
        let pos = rec.position().byte() as usize;
        let bytes_len = rec.all().len() + 1;

        match cluster_map.get(&rec.id) {}
        let Some(identifier) = cluster_map.get(&rec.id) else {
            if !skip_invalid_ids {
                bail!(RowNotInClusters { header: rec.id })
            }
            wtr.metadata.unmatched_read_count += 1;
            continue;
        };
        wtr.metadata.matched_read_count += 1;

        rec.id = identifier.clone();
        wtr.add_record(&rec, position, file_len, ignored)?;

        total_quality += rec.phred_quality_total();
        total_len += rec.len();
    }

    wtr.write_size((fastq_reader.position().byte() as f64) / (1024u32.pow(3) as f64));

    Ok(())
     */
}

/// Extract identifier components from a read header using a regex pattern
/// Returns the number of captures and the constructed identifier
fn extract_header_id(header: &str, re: &Regex, pos: usize) -> Result<(usize, RecordIdentifier)> {
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
    NoMatch {
        header: String,
        re: Regex,
        pos: usize,
    },

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
        pos: usize,
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
