use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder, Writer, WriterBuilder};
use regex::Regex;
use std::cell::RefCell;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::iter::Peekable;
use std::rc::Rc;

use crate::index::IndexGenerationErr::{InvalidClusterRow, RowNotInClusters};
use anyhow::{bail, Context, Result};
use needletail::parser::SequenceRecord;
use needletail::FastxReader;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::duplicates::RecordIdentifier;
use crate::file::ReadFileMetadata;
use crate::filter::{filter, FilterOpts};
use crate::io::Record;
use tempfile::tempfile_in;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexRecord {
    pub id: String,
    pub pos: usize,
    pub avg_qual: f64,
    pub n_bases: usize,
    pub rec_len: usize,
    pub ignored: bool,
}

pub struct IndexWriter {
    wtr: Writer<File>,
    temp_file: File,
    out_file: String,
    pub metadata: ReadFileMetadata,
}

impl IndexWriter {
    /// Create an IndexWriter from a desired output path. A temporary file is first used
    /// in order to store data, and will be created in the same directory as the output path.
    pub fn new(path: &str) -> Result<Self> {
        // get the directory of the output file
        let mut tempfile_dir = std::path::absolute(path)?;
        tempfile_dir.pop();

        // create a temporary file at this directory
        let temp_file = tempfile_in(tempfile_dir)?;

        let mut wtr = WriterBuilder::new()
            .delimiter(b'\t')
            .from_writer(temp_file.try_clone()?);

        Ok(IndexWriter {
            wtr,
            temp_file,
            out_file: path.to_string(),
            metadata: ReadFileMetadata {
                nailpolish_version: crate::cli::VERSION.to_string(),
                index_date: format!("{:?}", chrono::offset::Local::now()),
                ..ReadFileMetadata::default()
            },
        })
    }

    /// Finalizes the writing process by flushing the writer, writing metadata,
    /// and copying the temporary file contents to the final output file.
    pub fn finish_write(&mut self) -> Result<()> {
        info!("Writing to {}...", self.out_file);

        self.wtr.flush()?;

        // write to actual output file
        let mut wtr_out = File::create(&self.out_file)?;
        writeln!(wtr_out, "#{}", serde_json::to_string(&self.metadata)?)?;

        // drop the mutable write, and seek to the start so we can read
        // drop(self.wtr);
        self.temp_file.seek(std::io::SeekFrom::Start(0))?;

        // copy from the temporary file into the final output file
        std::io::copy(&mut self.temp_file, &mut wtr_out)?;

        Ok(())
    }

    /// Writes information about the Record to an external index writer, provided with
    /// extra information.
    ///
    /// # Arguments
    ///
    /// * `wtr` - A mutable reference to a CSV writer.
    /// * `pos` - The position of the record in the file.
    /// * `file_len` - The bytes consumed by the record in the file (the _length_ on _file_)
    pub fn write_record(
        &mut self,
        rec: &Record,
        pos: usize,
        file_len: usize,
        ignored: bool,
    ) -> csv::Result<()> {
        self.wtr.serialize(IndexRecord {
            id: rec.id.clone(),
            pos,
            avg_qual: rec.phred_quality_avg(),
            n_bases: rec.len(),
            rec_len: file_len,
            ignored,
        })
    }
}

pub struct IndexReader {
    path: String,
    pub(crate) metadata: ReadFileMetadata,
}

pub type IndexReaderRecords = DeserializeRecordsIntoIter<BufReader<File>, IndexRecord>;

impl IndexReader {
    pub fn from_path(path: &str) -> Result<Self> {
        let mut rdr = Self {
            path: path.to_string(),
            metadata: ReadFileMetadata::default(),
        };

        rdr.metadata = rdr.create_reader()?.0;

        Ok(rdr)
    }

    fn create_reader(&self) -> Result<(ReadFileMetadata, Reader<BufReader<File>>)> {
        let file = File::open(&self.path)?;
        let mut file = BufReader::new(file);

        let mut header = String::new();

        // read the first line, which is NOT in CSV format
        file.read_line(&mut header)
            .context("Could not read the first line")?;

        assert!(header.starts_with('#'));
        let metadata = serde_json::from_str(&header[1..])?;

        // Create CSV builder
        let rdr = ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(true)
            .from_reader(file);

        Ok((metadata, rdr))
    }

    /// Return the records of the index
    pub fn index_records(&mut self) -> Result<IndexReaderRecords> {
        let (_, mut rdr) = self.create_reader()?;
        Ok(rdr.into_deserialize())
    }
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
fn iter_lines_with_regex(
    reader: BufReader<File>,
    wtr: &mut IndexWriter,
    re: &Regex,
    skip_invalid_ids: bool,
    filter_opts: FilterOpts,
) -> Result<()> {
    // expected_len is used to ensure that every read has the same format
    let mut expected_len: Option<usize> = None;

    let mut fastq_reader = needletail::parser::FastqReader::new(reader);
    let mut total_quality = 0u32;
    let mut total_len = 0;

    while let Some(rec) = fastq_reader.next() {
        wtr.metadata.read_count += 1;

        if wtr.metadata.read_count % 50000 == 0 {
            info!("Processed: {}", wtr.metadata.read_count)
        }

        let sequence_rec = rec.expect("Invalid record");
        let position = sequence_rec.position().byte() as usize;
        let file_len = sequence_rec.all().len() + 1;
        let mut rec = Record::try_from(sequence_rec)?;

        // apply any filters
        let ignored = !filter(&rec, &filter_opts);
        wtr.metadata.filtered_reads += ignored as usize;

        let bc = extract_bc_from_header(&rec.id, re, position);

        // if this did not succeed...
        if let Err(e) = bc {
            if !skip_invalid_ids {
                bail!(e)
            }
            wtr.metadata.unmatched_read_count += 1;
            continue;
        }

        let (len, identifier) = bc?;

        // check that the number of barcode groups is the same
        let expected_len = *expected_len.get_or_insert(len);
        if expected_len != len {
            bail!(IndexGenerationErr::DifferentMatchCounts {
                header: rec.id,
                re: re.clone(),
                pos: position,
                count: len,
                expected: expected_len
            })
        }

        rec.id = identifier.to_string();

        wtr.write_record(&rec, position, file_len, ignored)?;
        total_quality += rec.phred_quality_total();
        total_len += rec.len();
        wtr.metadata.matched_read_count += 1;
    }

    wtr.metadata.avg_qual = (total_quality as f64) / (wtr.metadata.matched_read_count as f64);
    wtr.metadata.avg_len = (total_len as f64) / (wtr.metadata.matched_read_count as f64);
    wtr.metadata.gb = (fastq_reader.position().byte() as f64) / (1024u32.pow(3) as f64);

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
    wtr: &mut IndexWriter,
    clusters: &mut Reader<File>,
    skip_invalid_ids: bool,
    filter_opts: FilterOpts,
) -> Result<()> {
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

    // we store the total quality and length so that we can take an average at the end
    let mut total_quality = 0u32;
    let mut total_len = 0;

    while let Some(rec) = fastq_reader.next() {
        wtr.metadata.read_count += 1;

        // print progress notification
        if wtr.metadata.read_count % 50000 == 0 {
            info!("Processed: {}", wtr.metadata.read_count);
        }

        let sequence_rec = rec.expect("Invalid record");
        let position = sequence_rec.position().byte() as usize;
        let file_len = sequence_rec.all().len() + 1;
        let mut rec = Record::try_from(sequence_rec)?;

        // apply any filters
        let ignored = !filter(&rec, &filter_opts);
        wtr.metadata.filtered_reads += ignored as usize;

        let Some(identifier) = cluster_map.get(&rec.id) else {
            if !skip_invalid_ids {
                bail!(RowNotInClusters { header: rec.id })
            }
            wtr.metadata.unmatched_read_count += 1;
            continue;
        };
        wtr.metadata.matched_read_count += 1;

        rec.id = identifier.clone();
        wtr.write_record(&rec, position, file_len, ignored)?;

        total_quality += rec.phred_quality_total();
        total_len += rec.len();
    }

    // compute summary statistics
    wtr.metadata.avg_qual = (total_quality as f64) / (wtr.metadata.matched_read_count as f64);
    wtr.metadata.avg_len = (total_len as f64) / (wtr.metadata.matched_read_count as f64);
    wtr.metadata.gb = (fastq_reader.position().byte() as f64) / (1024u32.pow(3) as f64);

    Ok(())
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
fn extract_bc_from_header(
    header: &str,
    re: &Regex,
    pos: usize,
) -> Result<(usize, RecordIdentifier)> {
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

    Ok((
        captures.len(),
        RecordIdentifier {
            head: captures[0].to_string(),
            tail: captures[1..].join("_"),
        },
    ))
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
pub fn construct_index(
    infile: &str,
    outfile: &str,
    barcode_regex: &str,
    skip_unmatched: bool,
    clusters: &Option<String>,
    filter_opts: FilterOpts,
) -> Result<()> {
    // time everything!
    let now = std::time::Instant::now();

    // create the .fastq reader
    let f = File::open(infile).expect("File could not be opened");
    let reader = BufReader::new(f);

    // create the index file writer
    let mut wtr = IndexWriter::new(outfile)?;
    wtr.metadata.file_path = std::fs::canonicalize(infile)?.display().to_string();

    let re = Regex::new(barcode_regex)?;

    if let Some(filepath) = clusters {
        // parse identifier from a separate clusters file
        let mut cluster_rdr = csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(false)
            .from_path(filepath)?;

        iter_lines_with_cluster_file(
            reader,
            &mut wtr,
            &mut cluster_rdr,
            skip_unmatched,
            filter_opts,
        )?
    } else {
        // parse the identifier from the header
        iter_lines_with_regex(reader, &mut wtr, &re, skip_unmatched, filter_opts)?
    }

    // amount of time passed
    wtr.metadata.elapsed = now.elapsed().as_secs_f64();

    // report results
    if skip_unmatched {
        info!(
            "Stats: {} matched reads, {} unmatched reads, {} filtered reads, {:.1}s runtime",
            wtr.metadata.matched_read_count,
            wtr.metadata.unmatched_read_count,
            wtr.metadata.filtered_reads,
            wtr.metadata.elapsed,
        )
    } else {
        info!(
            "Stats: {} reads, {} filtered reads, {:.1}s runtime",
            wtr.metadata.matched_read_count, wtr.metadata.filtered_reads, wtr.metadata.elapsed
        )
    }

    wtr.finish_write()
}

#[derive(Error, Debug)]
enum IndexGenerationErr {
    #[error(
        "no matches produced:
position {pos}
    `{header}`
with capture group
    {re:?}
suggestion: inspect the read using `tail -c +{pos} <fastq> | head -n 5`
suggestion: if some of the reads should not produce a barcode, pass the --skip-unmatched flag"
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
