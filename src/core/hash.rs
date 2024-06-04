//! Cryptographic hashing of data
//!
//! Hashing is used everywhere in the blockchain infrastructure to prevent
//! tampering of the data.
//!

use crate::traits::io::{ByteIO, JsonIO};
use ethnum::U256;
use serde::de;
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug)]
pub struct HashDeserializeError;

impl fmt::Display for HashDeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash deserialization error")
    }
}

/// The hash structure is a 32 byte array representing the SHA256 hash value
/// of some arbitrary data.
///
/// It provides serialization/deserialization to and from both human readable
/// and binary formats
///
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Hash {
    value: [u8; Hash::SIZE],
}

impl Hash {
    const SIZE: usize = 32;

    pub fn new(data: &[u8]) -> Hash {
        Hash {
            value: Sha256::digest(data).as_slice().try_into().unwrap(),
        }
    }

    pub fn digest(&self) -> &[u8; Hash::SIZE] {
        &self.value
    }

    pub fn is_zero(&self) -> bool {
        U256::from_be_bytes(self.value.clone()) == U256::from(0_u32)
    }

    pub fn to_hex_str(&self) -> String {
        hex::encode(self.value)
    }

    pub fn from_hex_str(string: &str) -> Result<Hash, HashDeserializeError> {
        let data = match hex::decode(string) {
            Ok(value) => value,
            Err(_) => return Err(HashDeserializeError),
        };
        match data.as_slice().try_into() {
            Ok(value) => Ok(Hash { value }),
            Err(_) => Err(HashDeserializeError),
        }
    }
}

impl Default for Hash {
    fn default() -> Hash {
        Hash {
            value: [0; Hash::SIZE],
        }
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(self.to_hex_str().as_str())
        } else {
            let mut tup = serializer.serialize_tuple(self.value.len())?;
            for e in self.value {
                tup.serialize_element(&e)?;
            }
            tup.end()
        }
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            Hash::from_hex_str(&s).map_err(de::Error::custom)
        } else {
            let bytes: [u8; Hash::SIZE] = <[u8; Hash::SIZE]>::deserialize(deserializer)?;
            Ok(Hash { value: bytes })
        }
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_str())
    }
}

impl ByteIO for Hash {}
impl JsonIO for Hash {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest() {
        assert_eq!(
            Hash::new(b"test").digest(),
            &[
                159, 134, 208, 129, 136, 76, 125, 101, 154, 47, 234, 160, 197, 90, 208, 21, 163,
                191, 79, 27, 43, 11, 130, 44, 209, 93, 108, 21, 176, 240, 10, 8
            ]
        );
    }

    #[test]
    fn byte_io() {
        let bytes = vec![
            159, 134, 208, 129, 136, 76, 125, 101, 154, 47, 234, 160, 197, 90, 208, 21, 163, 191,
            79, 27, 43, 11, 130, 44, 209, 93, 108, 21, 176, 240, 10, 8,
        ];
        let hash = Hash::from_bytes(&bytes).unwrap();

        assert_eq!(Hash::new(b"test"), hash);
        assert_eq!(Hash::new(b"test").into_bytes(), Hash::new(b"test").digest());
    }

    #[test]
    fn is_zero() {
        assert!(Hash::default().is_zero());

        let bytes = vec![
            159, 134, 208, 129, 136, 76, 125, 101, 154, 47, 234, 160, 197, 90, 208, 21, 163, 191,
            79, 27, 43, 11, 130, 44, 209, 93, 108, 21, 176, 240, 10, 8,
        ];
        assert!(!Hash::from_bytes(&bytes).unwrap().is_zero());
    }

    #[test]
    fn hex() {
        let hash = Hash::new(b"test");

        assert_eq!(
            hash.to_hex_str(),
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
        assert_eq!(
            Hash::from_hex_str("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08")
                .unwrap(),
            hash,
        )
    }

    #[test]
    fn serialization() {
        let bytes = vec![
            159, 134, 208, 129, 136, 76, 125, 101, 154, 47, 234, 160, 197, 90, 208, 21, 163, 191,
            79, 27, 43, 11, 130, 44, 209, 93, 108, 21, 176, 240, 10, 8,
        ];
        let hex_str = "\"9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08\"";

        let hash_0 = Hash::from_bytes(&bytes).unwrap();
        assert_eq!(bytes, hash_0.value);
        assert_eq!(bytes, hash_0.into_bytes());

        let hash_1 = Hash::from_json(&hex_str).unwrap();
        assert_eq!(bytes, hash_1.value);
        assert_eq!(hex_str, hash_1.to_json().unwrap());

        assert_eq!(hash_0, hash_1);
    }
}
