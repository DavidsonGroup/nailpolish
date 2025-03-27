use crate::duplicates::{DuplicateMap, RecordIdentifier, RecordPosition};
use anyhow::{Context, Result};
use needletail::parser::SequenceRecord;
use needletail::{parse_fastx_reader, parser::FastqReader, FastxReader};
use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
// needed for write! to be implemented on Strings
use crate::index::{IndexReader, IndexReaderRecords, IndexRecord};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::iter::Map;
use std::rc::Rc;
use std::slice::Iter;

#[derive(PartialEq, Eq)]
pub enum ReadType {
    Consensus,
    Single,
    Original,
    Ignored,
}

#[derive(Clone)]
pub struct Record {
    pub id: String,
    pub seq: String,
    pub qual: String,
}

impl TryFrom<SequenceRecord<'_>> for Record {
    type Error = std::string::FromUtf8Error;

    /// Attempt to create a Record from a SequenceRecord.
    fn try_from(rec: SequenceRecord) -> Result<Self, Self::Error> {
        Ok(Record {
            id: String::from_utf8(rec.id().to_vec())?,
            seq: String::from_utf8(rec.seq().to_vec())?,
            qual: String::from_utf8(rec.qual().unwrap_or(&[]).to_vec())?,
        })
    }
}

impl Record {
    /// Returns the PHRED quality scores of the record as a byte slice.
    pub fn phred_quality(&self) -> Map<Iter<u8>, fn(&u8) -> u32> {
        // we transform the quality to a PHRED score (ASCII ! to I)
        // https://en.wikipedia.org/wiki/Phred_quality_score
        self.qual
            .as_bytes()
            .into_iter()
            .map(|&x| (x as u32) - 33u32)
    }

    /// Returns the average PHRED quality score of the record
    pub fn phred_quality_avg(&self) -> f64 {
        let qual = (self.phred_quality_total() as f64) / (self.len() as f64);
        // round to 2dp
        const ROUND_PRECISION: f64 = 100.0;
        (qual * ROUND_PRECISION).round() / ROUND_PRECISION
    }

    /// Returns the sum of the PHRED quality scores of the record
    pub fn phred_quality_total(&self) -> u32 {
        self.phred_quality().sum()
    }

    /// Returns the sequence length in base count of the record
    pub fn len(&self) -> usize {
        self.seq.len()
    }

    /// Write the Record in a .fastq format
    pub fn write_fastq(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        write!(writer, "@{}\n{}\n+\n{}", self.id, self.seq, self.qual)
    }

    /// Write the Record in a .fasta format
    pub fn write_fasta(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        write!(writer, ">{}\n{}", self.id, self.seq)
    }

    /// Adds metadata to the record identifier through an in-place modify.
    ///
    /// # Arguments
    ///
    /// * `umi_group` - The UMI group identifier.
    /// * `read_type` - The type of read (Consensus, Original, Ignored).
    /// * `group_idx` - The index of the read in the group.
    /// * `group_size` - The size of the group.
    /// * `avg_qual` - The average quality score of the group.
    ///
    /// # Note
    /// This function will modify the Record irreversibly by changing the Record's `id` field
    pub fn add_metadata(
        &mut self,
        umi_group: usize,
        read_type: ReadType,
        group_idx: usize,
        group_size: usize,
        avg_qual: f64,
    ) {
        let read_type_label = match read_type {
            ReadType::Consensus => &format!("CON_{group_size}"),
            ReadType::Single => "SIN",
            ReadType::Original => &format!("ORIG_{group_idx}_OF_{group_size}"),
            ReadType::Ignored => "IGN",
        };

        // safe to unwrap because this never returns an error
        //   https://github.com/rust-lang/rust/blob/1.47.0/library/alloc/src/string.rs#L2414-L2427
        // ">{} UG:i:{} BX:Z:{} UT:Z:{}_{}\n{}",
        write!(self.id, " UT:Z:{read_type_label} UG:i:{umi_group}")
            .expect("String writing should not error");

        // don't report the group average quality if the readtype is Original or Ignored
        if !matches!(read_type, ReadType::Original | ReadType::Ignored) {
            write!(self.id, " QL:f:{avg_qual:.2}").expect("String writing should not error");
        }
    }
}

pub struct UMIGroup {
    /// The "Identifier" of this group, typically a "BC_UMI" string
    pub id: RecordIdentifier,
    /// A 0-indexed integer unique to each UMI group
    pub index: usize,
    /// Each individual record within the UMI group
    pub records: Vec<Record>,
    /// The average PHRED quality of the UMI group
    pub avg_qual: f64,
    /// Whether we should NOT consensus call this UMI group, because of quality/other issues
    pub ignore: bool,
    pub consensus: Option<Record>,
}

pub struct UMIGroupCollection {
    seq_parser: Box<dyn FastxReader>,
    rnd_reader: File,
    index: IndexReader,
    duplicates: DuplicateMap,
    records: IndexReaderRecords,
}

impl UMIGroupCollection {
    pub fn new(mut index: IndexReader, input: &str) -> Result<Self> {
        let file = File::open(input).with_context(|| format!("Unable to open file {input}"))?;

        // create a sequential reader with a buffer size of BUF_CAPACITY
        const BUF_CAPACITY: usize = 1024usize.pow(2);
        let mut seq_reader = BufReader::with_capacity(BUF_CAPACITY, file);
        let mut seq_parser =
            parse_fastx_reader(seq_reader).context("Could not create fastx reader")?;

        // create a random access reader. we don't want a buffer as we plan to read a fixed amount of
        // bytes randomly
        let mut rnd_reader =
            File::open(input).with_context(|| format!("Unable to open file {input}"))?;

        let (duplicates, _) = index.get_duplicates()?;
        let records = index.index_records()?;

        Ok(UMIGroupCollection {
            seq_parser,
            rnd_reader,
            index,
            duplicates,
            records,
        })
    }

    /// Retrieves the next record from the sequence parser and the corresponding index record.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The sequence parser encounters an error while reading the next record.
    /// * The index reader encounters an error while reading the next index item.
    pub fn next_record(&mut self) -> Result<Option<(IndexRecord, SequenceRecord)>> {
        let Some(rec) = self.seq_parser.next() else {
            return Ok(None);
        };
        let rec = rec?;
        let idx = self
            .records
            .next()
            .context("No corresponding index record")??;

        Ok(Some((idx, rec)))
    }

    pub fn get_rec_random(&mut self, pos: &RecordPosition) -> Result<Record> {
        self.rnd_reader
            .seek(SeekFrom::Start(pos.pos as u64))
            .with_context(|| format!("Unable to seek file at position {}", pos.pos))?;

        // read the exact number of bytes
        let mut bytes = vec![0; pos.length];
        self.rnd_reader.read_exact(&mut bytes).with_context(|| {
            format!(
                "Could not read {} lines at position {}",
                pos.length, pos.pos
            )
        })?;

        // create a needletail 'reader' with the file at this location
        let mut fq_reader = FastqReader::new(&bytes[..]);

        let rec = fq_reader.next().context("Unexpected EOF")??;

        Record::try_from(rec).context("Could not perform utf8 conversions")
    }

    /// Creates a _streaming_ iterator over UMI groups in the collection.
    /// Since it is a streaming iterator, it does not support usual iterator methods
    /// and should be called using a `while let Some(v)...` loop.
    ///
    /// # Arguments
    ///
    /// * `duplicates_only` - A boolean indicating whether to process only duplicate reads.
    ///
    /// # Returns
    ///
    /// This function returns an iterator over `UMIGroupCollectionIter` which returns `UMIGroup`.
    pub fn stream_iter(&mut self, duplicates_only: bool) -> UMIGroupCollectionIter {
        UMIGroupCollectionIter {
            collection: self,
            visited_reads: HashSet::new(),
            duplicates_only,
            current_idx: 0,
        }
    }
}

pub struct UMIGroupCollectionIter<'a> {
    collection: &'a mut UMIGroupCollection,
    visited_reads: HashSet<usize>,
    duplicates_only: bool,
    current_idx: usize,
}

impl UMIGroupCollectionIter<'_> {
    /// Iterates over records in a FASTQ file by UMI group.
    ///
    /// # Returns
    /// This function returns an iterator of Results. When an Error is encountered,
    /// the caller should immediately stop. See the documentation for `until_err` to see an example.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The file cannot be opened.
    /// * The record cannot be read at the specified position.
    ///
    /// The iterator yields Some(Err) if:
    /// * There are issues reading the read at at the specified position. See the documentation for
    ///   `get_read_at_position` for more.
    pub fn next(&mut self) -> Result<Option<UMIGroup>> {
        let Some((idx, rec)) = self.collection.next_record()? else {
            return Ok(None);
        };
        // note: we don't need to add this to visited_reads, since traversal is in order
        let position = rec.position().byte() as usize;

        // if this is marked to ignore or we have already visited this, we can skip
        if self.visited_reads.contains(&position) || idx.ignored {
            return self.next();
        }

        let rec = Record::try_from(rec).context("Could not perform utf8 conversions")?;
        // get the corresponding entry in duplicates
        let id = RecordIdentifier::from_string(&idx.id);
        let group = self
            .collection
            .duplicates
            .records_by_pos(&position)
            .context("Could not find")?
            .clone();

        // skip over group sizes which are more than 1
        let group_size = group.len();
        if self.duplicates_only && group_size == 1 {
            return self.next();
        }

        let mut records = Vec::with_capacity(group_size);
        records.push(rec);

        // get all the other records as well - skip the first one, that's `rec`
        for pos in group.iter().skip(1) {
            self.visited_reads.insert(pos.pos);

            let rec = self.collection.get_rec_random(pos)?;
            records.push(rec)
        }

        let avg_qual =
            records.iter().map(|r| r.phred_quality_avg()).sum::<f64>() / (records.len() as f64);

        let umigroup = UMIGroup {
            id,
            index: self.current_idx,
            records,
            avg_qual,
            ignore: false,
            consensus: None,
        };
        self.current_idx += 1;

        Ok(Some(umigroup))
    }
}
