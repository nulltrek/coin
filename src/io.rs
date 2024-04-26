use crate::errors::DeserializeError;
use bincode;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

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
