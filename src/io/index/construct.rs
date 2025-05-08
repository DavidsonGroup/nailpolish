use super::filter::should_keep;
use super::storage::FileIndexPath;
use super::{DuplicateGroupKey, Index, ReadLocation, ReadLocationTrait, RecordIdentifier};
use crate::io::reads::QualityCompute;

use anyhow::{bail, Result};
use std::fs::File;
use std::io::{BufReader, Seek};
use std::path::{Path, PathBuf};
use thiserror::Error;

use needletail::parser::SequenceRecord;
use needletail::FastxReader;
use regex::Regex;

pub enum BarcodeLocation {
    Regex(String),
    ClusterFile(PathBuf),
}

/// Constructs an index from a FASTQ file and writes the results to an output file.
///
/// # Notes
/// This method will create a temporary file in the directory of the output file, and the OS
/// will automatically clean up this file after execution.
///
/// # Arguments
///
/// * `infile` - A string slice representing the path to the input FASTQ file.
/// * `outfile` - A string slice representing the path to the output file.
/// * `barcode_regex` - A string slice representing the regex pattern for extracting barcodes.
/// * `skip_unmatched` - A boolean indicating whether to skip unmatched reads.
/// * `clusters` - An optional string representing the path to the cluster file.
///
/// # Returns
///
/// Returns a `Result` indicating success or failure.
///
/// # Errors
///
/// This function will return an error if reading from the input file, writing to the output file,
/// or processing the data fails.
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

/// Iterates over lines in a FASTQ file, extracting barcodes using a regex
/// and writing the results to a CSV writer.
///
/// # Arguments
///
/// * `reader` - A `BufReader` for the input FASTQ file.
/// * `wtr` - A mutable reference to a CSV writer.
/// * `re` - A reference to a `Regex` for extracting barcodes from read headers.
/// * `skip_invalid_ids` - A boolean indicating whether to skip invalid IDs.
/// * `info` - A mutable `FastqFile` struct containing information about the FASTQ file.
///
/// # Returns
///
/// Returns a `Result` containing an updated `FastqFile` struct which contains information about
/// the file that was just read.
///
/// # Errors
///
/// This function will return an error if reading from the FASTQ file or writing to the CSV writer fails.
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

    // wtr.finish_read((fastq_reader.position().byte() as f64) / (1024u32.pow(3) as f64));
    Ok(())
}

/// Iterates over lines in a FASTQ file, matching read identifiers with a cluster file instead of
/// a header format, and writing the results to a CSV writer.
///
/// # Arguments
///
/// * `reader` - A `BufReader` for the input FASTQ file.
/// * `wtr` - A mutable reference to a CSV writer.
/// * `clusters` - A mutable reference to a CSV reader for the cluster file.
/// * `skip_invalid_ids` - A boolean indicating whether to skip invalid IDs.
/// * `info` - A mutable `FastqFile` struct containing information about the FASTQ file.
///
/// # Returns
///
/// Returns a `Result` containing an updated `FastqFile` struct which contains information about
/// the file that was just read.
///
/// # Errors
///
/// This function will return an error if reading from the FASTQ file, reading from the cluster file,
/// or writing to the CSV writer fails.
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

/// Extracts barcodes from a read header using a regex pattern.
///
/// # Arguments
///
/// * `header` - A string slice representing the read header.
/// * `re` - A reference to a `Regex` for extracting barcodes from the header.
/// * `pos` - The position of the read.
///
/// # Returns
///
/// Returns a `Result` containing a tuple with the number of captures and the
/// concatenated barcode string (identifier).
///
/// # Errors
///
/// This function will return an error if the regex does not match the header.
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
