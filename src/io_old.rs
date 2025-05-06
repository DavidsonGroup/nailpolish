use crate::duplicates::{DuplicateMap, RecordIdentifier, RecordPosition};
use anyhow::{Context, Result};
use needletail::parser::SequenceRecord;
use needletail::{parse_fastx_reader, parser::FastqReader, FastxReader};
use std::cmp::PartialEq;
use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
// needed for write! to be implemented on Strings
use crate::r#mod::ReadStatus::Filtered;
use crate::r#mod::{ArchivedIndex, Index, IndexReader, IndexedReadType, ReadLocation};
use crate::reader::UncompressedFileReader;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::iter::Map;
use std::path::{Path, PathBuf};
use std::slice::Iter;

#[derive(PartialEq, Eq, Clone)]
pub enum ReadType {
    Consensus,
    Single,
    Original,
    Ignored,
}

#[derive(Clone)]
pub struct RecordMetadata {
    umi_group: usize,
    read_type: ReadType,
    group_idx: usize,
    group_size: usize,
}

/// Represents an in-memory record of a sequence. Unlike a SequenceRecord, the contents of this
/// sequence are fully owned and held within the struct. This means that it is threadsafe.
#[derive(Clone)]
pub struct InMemoryRecord {
    /// The identifier of the record.
    pub header: String,
    /// The sequence of the record.
    pub seq: String,
    /// The quality scores of the record.
    pub qual: String,
    pub metadata: Option<RecordMetadata>,
}

impl TryFrom<SequenceRecord<'_>> for InMemoryRecord {
    type Error = std::string::FromUtf8Error;

    /// Attempt to create a Record from a SequenceRecord.
    fn try_from(rec: SequenceRecord) -> Result<Self, Self::Error> {
        Ok(InMemoryRecord {
            header: String::from_utf8(rec.id().to_vec())?,
            seq: String::from_utf8(rec.seq().to_vec())?,
            qual: String::from_utf8(rec.qual().unwrap_or(&[]).to_vec())?,
            metadata: None,
        })
    }
}

impl InMemoryRecord {
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
        write!(writer, "@{}\n{}\n+\n{}", self.header, self.seq, self.qual)
    }

    /// Write the Record in a .fasta format
    pub fn write_fasta(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        write!(writer, ">{}\n{}", self.header, self.seq)
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
        write!(self.header, " UT:Z:{read_type_label} UG:i:{umi_group}")
            .expect("String writing should not error");

        // don't report the group average quality if the readtype is Original or Ignored
        if !matches!(read_type, ReadType::Original | ReadType::Ignored) {
            write!(self.header, " QL:f:{avg_qual:.2}").expect("String writing should not error");
        }
    }
}

pub struct DuplicateGroup {
    /// The "Identifier" of this group, typically a "BC_UMI" string
    pub id: RecordIdentifier,
    /// A 0-indexed integer unique to each UMI group
    pub index: usize,
    /// Each individual record within the UMI group
    pub records: Vec<InMemoryRecord>,
    /// The average PHRED quality of the UMI group
    pub avg_qual: f64,
    /// Whether we should NOT consensus consensus this UMI group, because of quality/other issues
    pub ignore: bool,
    pub consensus: Option<InMemoryRecord>,
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

    pub fn get_rec_random(&mut self, pos: &RecordPosition) -> Result<InMemoryRecord> {
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

        InMemoryRecord::try_from(rec).context("Could not perform utf8 conversions")
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
    pub fn next(&mut self) -> Result<Option<DuplicateGroup>> {
        let Some((idx, rec)) = self.collection.next_record()? else {
            return Ok(None);
        };
        // note: we don't need to add this to visited_reads, since traversal is in order
        let position = rec.position().byte() as usize;

        // if this is marked to ignore or we have already visited this, we can skip
        if self.visited_reads.contains(&position) || idx.ignored {
            return self.next();
        }

        let rec = InMemoryRecord::try_from(rec).context("Could not perform utf8 conversions")?;
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

        let umigroup = DuplicateGroup {
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

pub struct FileIndex {
    path: FileIndexPath,
    index_reader: IndexReader,
    index: dyn AsRef<Index>,
    reader: UncompressedFileReader,
    _visited_reads: HashSet<usize>,
    _counter: usize,
}

impl FileIndex {
    pub fn new(fastq: &Path) -> Result<Self> {
        let path = FileIndexPath::new(fastq);
        let index_reader = IndexReader::load(path.clone())?;
        let index = index_reader.index();

        const BUF_CAPACITY: usize = 1024usize.pow(2);
        let mut reader = UncompressedFileReader::new(path.fastq())?;
        let _visited_reads = HashSet::new();

        Ok(Self {
            path,
            index_reader,
            index,
            reader,
            _visited_reads,
            _counter: 0,
        })
    }

    pub fn next_candidate_group(&mut self, duplicates_only: bool) -> Result<Option<()>> {
        let (read_entry, rec) = match self.next_read()? {
            Some(v) => v,
            None => return Ok(None),
        };

        // do we need to keep looking?
        if read_entry.status == Filtered {}

        let group = self.index.as_ref().get_group_of_read(read_entry)?;
        let (id, reads) = group;

        todo!();
    }

    fn next_read(&mut self) -> Result<Option<(&ReadLocation, SequenceRecord)>> {
        let (read_idx, rec) = match self.reader.next()? {
            Some(v) => v,
            None => return Ok(None),
        };
        let pos = rec.position().byte() as usize;

        // check if this read has already been visited
        if self._visited_reads.contains(&pos) {
            return self.next_read();
        }

        let read = self.index.as_ref().get_read_by_index(read_idx)?;

        Ok(Some((read, rec)))
    }

    fn get_group(
        &mut self,
        result: Result<(usize, SequenceRecord), needletail::errors::ParseError>,
    ) -> Result<DuplicateGroup> {
        let (idx, rec) = result?;
        let (group_idx, type_) = &self.index.as_ref().reads()[idx];
        let position = rec.position().byte() as usize;

        // if this is marked to ignore or we have already visited this, we can skip
        if self._visited_reads.contains(&position) || type_ == IndexedReadType::Ignored {
            return self._next_group();
        }

        let group = &self
            .index
            .as_ref()
            .get_group_by_index(*group_idx)
            .context("Could not find corresponding group")?;

        // skip over group sizes which are only 1 (i.e. not a duplicate)
        let group_size = group.len();
        // if self.duplicates_only && group_size == 1 {
        //     return self._next_group();
        // }

        let rec = InMemoryRecord::try_from(rec)?;
        let mut records = Vec::with_capacity(group_size);
        records.push(rec);

        // get all the other records as well - skip the first one, that's `rec`
        for pos in group.iter().skip(1) {
            self._visited_reads.insert(pos.pos);

            let rec = InMemoryRecord::try_from(self.reader.get_rec_random(pos)?)?;
            records.push(rec)
        }

        let avg_qual =
            records.iter().map(|r| r.phred_quality_avg()).sum::<f64>() / (records.len() as f64);

        let umigroup = DuplicateGroup {
            id,
            index: self._counter,
            records,
            avg_qual,
            ignore: false,
            consensus: None,
        };
        self._counter += 1;

        Ok(umigroup)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn compute_index() {
        assert_eq!(
            compute_index_path(&Path("test.fastq")),
            PathBuf("test.fastq.idx")
        )
    }
}
