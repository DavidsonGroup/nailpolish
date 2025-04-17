//! `io::index::storage`
//!
//! This module is responsible for storing the index object on the filesystem.
//! It provides functionality to serialize, deserialize, and manage index data
//! for efficient storage and retrieval.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Handles storing the index object on the filesystem.
pub struct IndexStorage;

impl IndexStorage {
    /// Saves the index object to the specified file path.
    ///
    /// # Arguments
    ///
    /// * `index` - The index object to be stored.
    /// * `path` - The file path where the index will be saved.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(index: &str, path: &Path) -> io::Result<()> {
        let mut file = fs::File::create(path)?;
        file.write_all(index.as_bytes())?;
        Ok(())
    }

    /// Loads the index object from the specified file path.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path from which the index will be loaded.
    ///
    /// # Returns
    ///
    /// Returns the loaded index object as a `String`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn load(path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }
}
