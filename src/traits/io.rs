use bincode;
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::io::{Read, Write};

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
    fn from_file(file: &mut File) -> Result<Self, FileIOError> {
        let mut buffer = Vec::new();
        let result = file.read_to_end(&mut buffer);
        if result.is_err() {
            return Err(FileIOError);
        }

        let result = match Self::from_bytes(buffer.as_slice()) {
            Ok(item) => Ok(item),
            Err(_) => Err(FileIOError),
        };
        result
    }

    fn to_file(self: &Self, file: &mut File) -> Result<usize, FileIOError> {
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
