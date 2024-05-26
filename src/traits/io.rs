use bincode;
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug)]
pub struct SerializeError;

#[derive(Debug)]
pub struct DeserializeError;

impl fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Deserialization error")
    }
}

pub trait ByteIO: Serialize + for<'a> Deserialize<'a> {
    fn from_bytes(bytes: &[u8]) -> Result<Self, DeserializeError> {
        match bincode::deserialize(&bytes) {
            Ok(item) => Ok(item),
            Err(_) => Err(DeserializeError),
        }
    }

    fn into_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}

#[derive(Debug)]
pub struct FileIOError;

pub trait FileIO: Sized + ByteIO {
    fn from_file(path: &Path) -> Result<Self, FileIOError> {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return Err(FileIOError),
        };

        Self::from_file_descriptor(&mut file)
    }

    fn from_file_descriptor(file: &mut File) -> Result<Self, FileIOError> {
        let mut buffer = Vec::new();
        let result = file.read_to_end(&mut buffer);
        if result.is_err() {
            return Err(FileIOError);
        }

        match Self::from_bytes(buffer.as_slice()) {
            Ok(item) => Ok(item),
            Err(_) => Err(FileIOError),
        }
    }

    fn to_file(self: &Self, path: &Path) -> Result<usize, FileIOError> {
        let mut file = match File::create(path) {
            Ok(file) => file,
            Err(_) => return Err(FileIOError),
        };

        self.to_file_descriptor(&mut file)
    }

    fn to_file_descriptor(self: &Self, file: &mut File) -> Result<usize, FileIOError> {
        let bytes = self.into_bytes();
        match file.write_all(&bytes) {
            Ok(_) => Ok(bytes.len()),
            Err(_) => Err(FileIOError),
        }
    }
}

pub trait JsonIO: Sized + Serialize + for<'a> Deserialize<'a> {
    fn to_json(self: &Self) -> Result<String, SerializeError> {
        match serde_json::to_string(self) {
            Ok(value) => Ok(value),
            Err(_) => Err(SerializeError),
        }
    }

    fn from_json(string: &str) -> Result<Self, DeserializeError> {
        match serde_json::from_str(string) {
            Ok(value) => Ok(value),
            Err(_) => Err(DeserializeError),
        }
    }
}
