//! Traits for dealing with serialization and IO
//!
//! These traits provide standard functions for serializing/deserializing
//! structs into byte arrays or JSON and for saving the data into files.
//!
//! Serialization/deserialization is done through the serde crate
//!

use bincode;
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Error enum for defining very high lever error codes
#[derive(Debug)]
pub enum IOError {
    SerializationFailed,
    DeserializationFailed,
    FileOperationFailed,
}

impl fmt::Display for IOError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IO error: {}",
            match self {
                IOError::SerializationFailed => "serialization failed",
                IOError::DeserializationFailed => "deserialization failed",
                IOError::FileOperationFailed => "file operation failed",
            }
        )
    }
}

/// Implements ser/de to/from byte arrays
///
pub trait ByteIO: Serialize + for<'a> Deserialize<'a> {
    fn from_bytes(bytes: &[u8]) -> Result<Self, IOError> {
        match bincode::deserialize(&bytes) {
            Ok(item) => Ok(item),
            Err(_) => Err(IOError::DeserializationFailed),
        }
    }

    fn into_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}

/// Implements functions for writing/reading a byte array to/from a file
///
pub trait FileIO: Sized + ByteIO {
    fn from_file(path: &Path) -> Result<Self, IOError> {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return Err(IOError::FileOperationFailed),
        };

        Self::from_file_descriptor(&mut file)
    }

    fn from_file_descriptor(file: &mut File) -> Result<Self, IOError> {
        let mut buffer = Vec::new();
        let result = file.read_to_end(&mut buffer);
        if result.is_err() {
            return Err(IOError::FileOperationFailed);
        }

        match Self::from_bytes(buffer.as_slice()) {
            Ok(item) => Ok(item),
            Err(_) => Err(IOError::FileOperationFailed),
        }
    }

    fn to_file(self: &Self, path: &Path) -> Result<usize, IOError> {
        let mut file = match File::create(path) {
            Ok(file) => file,
            Err(_) => return Err(IOError::FileOperationFailed),
        };

        self.to_file_descriptor(&mut file)
    }

    fn to_file_descriptor(self: &Self, file: &mut File) -> Result<usize, IOError> {
        let bytes = self.into_bytes();
        match file.write_all(&bytes) {
            Ok(_) => Ok(bytes.len()),
            Err(_) => Err(IOError::FileOperationFailed),
        }
    }
}

/// Implements ser/de to/from JSON using
///
pub trait JsonIO: Sized + Serialize + for<'a> Deserialize<'a> {
    fn to_json(self: &Self) -> Result<String, IOError> {
        match serde_json::to_string(self) {
            Ok(value) => Ok(value),
            Err(_) => Err(IOError::SerializationFailed),
        }
    }

    fn from_json(string: &str) -> Result<Self, IOError> {
        match serde_json::from_str(string) {
            Ok(value) => Ok(value),
            Err(_) => Err(IOError::DeserializationFailed),
        }
    }
}
