use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder, Writer, WriterBuilder};
use regex::Regex;
use std::cell::RefCell;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::iter::{Filter, Map, Peekable};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::r#mod::IndexGenerationErr::{InvalidClusterRow, RowNotInClusters};
use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use indexmap::map::Iter;
use needletail::parser::SequenceRecord;
use needletail::FastxReader;
use rkyv::{deserialize, Archive, Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use thiserror::Error;

use crate::duplicates::RecordIdentifier;
use crate::file::ReadFileMetadata;
use crate::filter::{filter, FilterOpts, QualityCompute};
use crate::io::{FileIndexPath, InMemoryRecord};

pub type GroupIndex = usize;

// An individual indexed read, with its byte length in file
// and read status
#[derive(Archive, Serialize, Deserialize, Clone)]
pub struct ReadLocation {
    pub pos: usize,
    pub byte_len: usize,
}

/// Generated on demand when iterating through the index.
pub struct DuplicateGroup<'a> {
    pub key: &'a DuplicateGroupKey,
    pub reads: &'a SmallVec<[ReadLocation; 1]>,
    pub index: usize
}

/// Represents the index of a duplicate group.
///
/// - `Normal`: A group with a valid `RecordIdentifier`.
/// - `Ignored` and `Filtered`: A group that is ignored, represented by a unique value. This can be
///   the read's byte position. It is important that this is guaranteed unique, as this prevents
///   collisions and overwrites from occurring.
#[derive(Archive, Serialize, Hash, Eq, PartialEq)]
enum DuplicateGroupKey {
    Normal(RecordIdentifier),
    Invalid(usize),
    Filtered(usize)
}


/// Represents an index structure for grouping and organizing reads as well as duplicate groups.
/// Reads are labelled
/// The `Index` struct contains the following fields:
/// - `groups`: Maps a `RecordIdentifier` to a collection of read numbers (`GroupReads`).
/// - `reads`: A vector of tuples where each tuple contains the byte length of a read and its type (`IndexedReadType`).
/// - `metadata`: Metadata about the reads, such as counts and quality metrics.
#[derive(Default, Archive, Deserialize, Serialize)]
pub struct Index {
    // idea: Key enables fast lookup of reads BUT there can be
    groups: IndexMap<DuplicateGroupKey, SmallVec<[ReadLocation; 1]>>,
    pub metadata: ReadFileMetadata,
}

impl Index {
    // pub fn groups<'a>(&'a self) -> Map<Iter<'_, DuplicateGroupKey, StoredDuplicateGroup>, fn((&'a DuplicateGroupKey, &'a StoredDuplicateGroup)) -> DuplicateGroup<'a>> {
    pub fn groups<'a>(&'a self) -> impl Iterator<Item = DuplicateGroup> + 'a {
        self.groups.iter()
            .enumerate()
            .map(|(index, (key, value))| {
                DuplicateGroup {
                    key,
                    reads: value,
                    index
                }
            })
    }

    pub fn get_by_key(&self, key: &DuplicateGroupKey) -> Option<DuplicateGroup> {
        self.groups.get_full(key)
            .map(|(index, key, reads)| {
                DuplicateGroup {
                    key,
                    reads,
                    index
                }
            })
    }

    pub fn get_by_record_id(&self, id: RecordIdentifier) -> Option<DuplicateGroup> {
        self.get_by_key(&DuplicateGroupKey::Normal(id))
    }

    pub fn add_read(&mut self, key: DuplicateGroupKey, read: ReadLocation) {
        self.groups.entry(key)
            .and_modify(|e| e.push(read.clone()))
            .or_insert_with(|| smallvec![read]);
    }

    pub fn metadata(&self) -> &ReadFileMetadata {
        &self.metadata
    }
}

pub struct IndexReader {
    file: FileIndexPath,
    contents: Vec<u8>,
}

impl IndexReader {
    pub fn load(file: FileIndexPath) -> Result<Self> {
        let contents = std::fs::read(file.index())?;

        Ok(Self { file, contents })
    }

    pub fn index(&self) -> ArchivedIndex {
        rkyv::access<ArchivedIndex, rkyv::rancor::Error>(&self.contents).unwrap()
    }
}

pub struct IndexWriter {
    file: FileIndexPath,
    index: Index,
    filters: FilterOpts,
    _now: std::time::Instant,
}

impl IndexWriter {
    /// Create an IndexWriter with a desired output path.
    pub fn new(file: FileIndexPath, filters: FilterOpts) -> Result<Self> {
        let metadata = ReadFileMetadata {
            nailpolish_version: crate::cli::VERSION.to_string(),
            index_date: format!("{:?}", chrono::offset::Local::now()),
            file_path: std::fs::canonicalize(file.fastq())?.display().to_string(),
            ..ReadFileMetadata::default()
        };

        let index = Index {
            metadata,
            ..Index::default()
        };

        Ok(IndexWriter {
            file,
            index,
            filters,
            _now: std::time::Instant::now(),
        })
    }

    /// Writes information about the Record to an external index writer, provided with
    /// extra information.
    ///
    /// # Arguments
    ///
    /// * `wtr` - A mutable reference to a CSV writer.
    /// * `pos` - The position of the record in the file.
    /// * `file_len` - The bytes consumed by the record in the file (the _length_ on _file_)
    pub fn add_record(
        &mut self,
        id: Option<RecordIdentifier>,
        read: SequenceRecord,
        read_location: ReadLocation
    ) -> csv::Result<()> {
        self.index.metadata.read_count += 1;

        // make update messages?
        if self.index.metadata.read_count % 50000 == 0 {
            info!("Processed: {}", self.index.metadata.read_count)
        }

        // id is None means that RecordIdentifier was not found
        let Some(id) = id else {
            self.index.metadata.unmatched_read_count += 1;
            return Ok(());
        };

        let key = match id {
            Some(id) => {
                if filter(&read, &self.filters) {
                    self.index.metadata.filtered_reads += 1;
                    DuplicateGroupKey::Filtered(read_location.pos)
                } else {
                    self.index.metadata.matched_read_count += 1;
                    DuplicateGroupKey::Normal(id)
                }
            }
            None => {
                self.index.metadata.unmatched_read_count += 1;
                DuplicateGroupKey::Invalid(read_location.pos)
            }
        };

        // metadata changes
        self.index.metadata.total_len += read.num_bases() as f64;
        self.index.metadata.total_qual = read.phred_quality_avg();

        // add the read to groups
        self.index.add_read(key, read_location);

        Ok(())
    }

    pub fn metadata(&self) -> &ReadFileMetadata {
        self.index.metadata()
    }

    pub fn finish_read(&mut self, size_gb: f64) {
        self.index.metadata.gb = size_gb;
        self.index.metadata.elapsed = self._now.elapsed().as_secs_f64();
    }

    /// Finalizes the writing process by flushing the writer, writing metadata,
    /// and copying the temporary file contents to the final output file.
    pub fn write(&mut self) -> Result<()> {
        info!("Writing to {}...", self.file.index().display());

        let mut output = File::open(self.file.index())?;
        let mut io_writer = rkyv::ser::writer::IoWriter::new(output);

        self.index.serialize(&mut io_writer)?;
        Ok(())
    }
}

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
pub fn construct_index(
    fastq: &Path,
    barcode_location: BarcodeLocation,
    skip_unmatched: bool,
    filters: FilterOpts,
) -> Result<()> {
    let file = FileIndexPath::new(fastq);
    let mut wtr = IndexWriter::new(file.clone(), filters)?;

    // create the .fastq reader
    let f = File::open(file.fastq()).expect("File could not be opened");
    let reader = BufReader::new(f);

    match barcode_location {
        BarcodeLocation::Regex(re) => {
            let re = Regex::new(&re)?;
            iter_lines_with_regex(reader, &mut wtr, &re, skip_unmatched)?
        }
        BarcodeLocation::ClusterFile(file) => {
            iter_lines_with_cluster_file(reader, &mut wtr, &file, skip_unmatched)?
        }
    }

    // report results
    let metadata = wtr.metadata();
    if skip_unmatched {
        info!(
            "Stats: {} matched reads, {} unmatched reads, {} filtered reads, {:.1}s runtime",
            metadata.matched_read_count,
            metadata.unmatched_read_count,
            metadata.filtered_reads,
            metadata.elapsed,
        )
    } else {
        info!(
            "Stats: {} reads, {} filtered reads, {:.1}s runtime",
            metadata.matched_read_count, metadata.filtered_reads, metadata.elapsed
        )
    }

    wtr.write()
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
) -> Result<()> {
    // expected_len is used to ensure that every read has the same format
    let mut expected_len: Option<usize> = None;

    let mut fastq_reader = needletail::parser::FastqReader::new(reader);

    while let Some(rec) = fastq_reader.next() {
        let rec = rec.expect("Invalid record");

        let read_location = ReadLocation {
            pos: rec.position().byte() as usize,
            byte_len: rec.all().len() + 1
        };
        let header = std::str::from_utf8(rec.id())?;

        let bc = extract_header_id(header, re, read_location.pos);
        match bc {
            Ok((len, id)) => {
                // check # of barcode groups is the same
                let expected_len = *expected_len.get_or_insert(len);
                if expected_len != len {
                    bail!(IndexGenerationErr::DifferentMatchCounts {
                        header: header.to_string(),
                        re: re.clone(),
                        pos,
                        count: len,
                        expected: expected_len
                    })
                }

                wtr.add_record(Some(id), rec, read_location)?
            }
            Err(e) => {
                if !skip_invalid_ids {
                    bail!(e)
                }

                wtr.add_record(None, rec, read_location)?
            }
        }
    }

    wtr.finish_read((fastq_reader.position().byte() as f64) / (1024u32.pow(3) as f64));

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

    Ok((
        captures.len(),
        RecordIdentifier {
            head: captures[0].to_string(),
            tail: captures[1..].join("_"),
        },
    ))
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
