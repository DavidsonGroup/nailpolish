// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use anyhow::Result;

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
        name_lower.ends_with(".gz") || 
        name_lower.ends_with(".fastq.gz") || 
        name_lower.ends_with(".fq.gz")
    } else {
        false
    }
}
