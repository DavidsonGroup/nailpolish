// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use anyhow::Result;

/// A half-open byte range `[start, end)` within the uncompressed data stream.
#[derive(Copy, Clone)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}

impl ByteRange {
    /// Length of the range in bytes, as expected by [`Accessor::read`].
    pub(crate) fn len(&self) -> u32 {
        (self.end - self.start) as u32
    }
}

/// Calculates a running average when a new value is added to an existing average
pub(crate) fn running_avg(existing: f32, new: f32, new_count: usize) -> f32 {
    let new_count = new_count as f32;
    (existing * (new_count - 1.0) + new) / new_count
}

/// Computes the arithmetic mean of a sequence of f32 values
pub(crate) fn complete_avg<I>(iter: I) -> f32
where
    I: Iterator<Item = f32>,
{
    let (sum, count) = iter.fold((0.0, 0), |(sum, count), val| (sum + val, count + 1));
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}

/// Formats an integer with `,` thousands separators, e.g. `3030565` -> `3,030,565`.
pub(crate) fn fmt_count(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);

    // the leading group holds the leftover digits; every group after it is three
    // wide. digits are always ASCII, so splitting on byte offsets is safe.
    let first = (digits.len() - 1) % 3 + 1;
    let (head, mut rest) = digits.split_at(first);
    out.push_str(head);

    while !rest.is_empty() {
        let (chunk, tail) = rest.split_at(3);
        out.push(',');
        out.push_str(chunk);
        rest = tail;
    }

    out
}

/// Returns a writer to the given path or stdout if no path is provided.
pub(crate) fn get_writer(path: Option<&Path>) -> Result<Box<dyn std::io::Write>> {
    let writer: Box<dyn std::io::Write> = match path {
        Some(v) => {
            let wtr = BufWriter::with_capacity(*crate::env::WRITE_BUF_CAPACITY, File::create(v)?);
            Box::new(wtr)
        }
        None => {
            let wtr = BufWriter::with_capacity(*crate::env::WRITE_BUF_CAPACITY, std::io::stdout());
            Box::new(wtr)
        }
    };

    Ok(writer)
}

pub(crate) fn deserialize_standard<T>(
    archived_val: &impl rkyv::Deserialize<T, rkyv::api::high::HighDeserializer<rkyv::rancor::Error>>,
) -> T {
    rkyv::deserialize::<T, rkyv::rancor::Error>(archived_val).unwrap()
}

/// Checks if a file path appears to be gzip compressed based on its extension
pub(crate) fn is_gzip_file(path: &Path) -> bool {
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        let name_lower = file_name.to_lowercase();
        name_lower.ends_with(".gz")
            || name_lower.ends_with(".fastq.gz")
            || name_lower.ends_with(".fq.gz")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::fmt_count;

    #[test]
    fn fmt_count_groups_digits_from_the_right() {
        // group boundaries: each length mod 3 has a different leading group
        assert_eq!(fmt_count(0), "0");
        assert_eq!(fmt_count(7), "7");
        assert_eq!(fmt_count(12), "12");
        assert_eq!(fmt_count(123), "123");
        assert_eq!(fmt_count(1234), "1,234");
        assert_eq!(fmt_count(12345), "12,345");
        assert_eq!(fmt_count(123456), "123,456");
        assert_eq!(fmt_count(1234567), "1,234,567");
        assert_eq!(fmt_count(3030565), "3,030,565");
        // zeroes within a group must be preserved
        assert_eq!(fmt_count(1000000), "1,000,000");
    }
}
