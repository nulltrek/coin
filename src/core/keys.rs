//! Private/Public key implementation base on Elliptic Curve Cryptography
//!
//! Private keys are used for signing transactions, thus verifying ownership of coins.
//! Public keys are used for verifying the validity of signatures, and they act as
//! addresses to send coins to.
//!

use crate::traits::io::{ByteIO, DeserializeError, FileIO};
use ed25519_dalek::{
    Signature as DalekSignature, Signer, SigningKey, Verifier as DalekVerifier, VerifyingKey,
    PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH,
};
use hex;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt;

pub trait Verifier {
    fn verify(&self, message: &[u8], signature: &Signature) -> bool;
}

/// A private key representation
pub type PrivateKey = [u8; SECRET_KEY_LENGTH];

#[derive(Debug)]
pub struct PubkeyDeserializeError;

/// A public key representation.
///
/// Functions are provided for verifying signatures and for serialization.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct PublicKey {
    value: [u8; PUBLIC_KEY_LENGTH],
}

impl PublicKey {
    pub fn to_hex_str(&self) -> String {
        hex::encode(self.value)
    }

    pub fn from_hex_str(string: &str) -> Result<PublicKey, PubkeyDeserializeError> {
        let data = match hex::decode(string) {
            Ok(value) => value,
            Err(_) => return Err(PubkeyDeserializeError),
        };
        match data.as_slice().try_into() {
            Ok(value) => Ok(PublicKey { value }),
            Err(_) => Err(PubkeyDeserializeError),
        }
    }
}

impl Verifier for PublicKey {
    fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        let verifying_key = match VerifyingKey::from_bytes(&self.value) {
            Ok(key) => key,
            Err(_) => return false,
        };
        verifying_key.verify(message, &signature.0).is_ok()
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_str())
    }
}

/// A signature representation
///
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Signature(DalekSignature);

/// A key pair representation
///
/// It can both sign data and verify such signatures.
///
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct KeyPair(SigningKey);

impl KeyPair {
    pub fn new() -> KeyPair {
        let mut csprng = OsRng;
        KeyPair(SigningKey::generate(&mut csprng))
    }

    pub fn private_key(&self) -> PrivateKey {
        self.0.as_bytes().clone()
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            value: self.0.verifying_key().to_bytes(),
        }
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        Signature(self.0.sign(message))
    }
}

impl Verifier for KeyPair {
    fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        match self.0.verify(message, &signature.0) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

impl ByteIO for KeyPair {
    fn from_bytes(bytes: &[u8]) -> Result<KeyPair, DeserializeError> {
        if bytes.len() != 32 {
            return Err(DeserializeError);
        }
        let result: Result<&[u8; 32], core::array::TryFromSliceError> = bytes.try_into();
        match result {
            Ok(slice) => Ok(KeyPair(SigningKey::from_bytes(slice))),
            Err(_) => Err(DeserializeError),
        }
    }

    fn into_bytes(&self) -> Vec<u8> {
        Vec::from(self.private_key())
    }
}

impl FileIO for KeyPair {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::hash::Hash;
    use tempfile::*;

    #[test]
    fn signing() {
        let key = KeyPair::new();
        let signature = key.sign(b"test");
        assert_eq!(key.verify(b"test", &signature), true);
    }

    #[test]
    fn serialize() {
        let bytes = [0u8; 32];
        let pair = KeyPair::from_bytes(bytes.as_slice()).unwrap();
        let serialized = pair.into_bytes();
        assert_eq!(serialized, bytes);
    }

    #[test]
    fn deserialize() {
        let bytes = vec![0u8; 32];
        assert!(match KeyPair::from_bytes(bytes.as_slice()) {
            Ok(_) => true,
            Err(_) => false,
        });

        let bytes = vec![0u8; 31];
        assert!(match KeyPair::from_bytes(bytes.as_slice()) {
            Ok(_) => false,
            Err(_) => true,
        });

        let bytes = vec![0u8; 34];
        assert!(match KeyPair::from_bytes(bytes.as_slice()) {
            Ok(_) => false,
            Err(_) => true,
        });
    }

    #[test]
    fn hex() {
        let pubkey = PublicKey {
            value: [
                159, 134, 208, 129, 136, 76, 125, 101, 154, 47, 234, 160, 197, 90, 208, 21, 163,
                191, 79, 27, 43, 11, 130, 44, 209, 93, 108, 21, 176, 240, 10, 8,
            ],
        };

        assert_eq!(
            pubkey.to_hex_str(),
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );

        assert_eq!(
            PublicKey::from_hex_str(
                "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
            )
            .unwrap(),
            pubkey,
        )
    }

    #[test]
    fn file_io() {
        let original = KeyPair::new();

        let temp_file = NamedTempFile::new().unwrap();

        let mut out_file = temp_file.reopen().unwrap();
        let result = original.to_file_descriptor(&mut out_file);
        assert!(result.is_ok());

        let mut in_file = temp_file.reopen().unwrap();
        let deserialized = KeyPair::from_file_descriptor(&mut in_file).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn verify_test() {
        let key_1 = KeyPair::new();
        let hash = Hash::new(b"test");
        let signature = key_1.sign(hash.digest());

        assert!(key_1.public_key().verify(hash.digest(), &signature));

        let key_2 = KeyPair::new();
        assert!(!key_2.public_key().verify(hash.digest(), &signature));
    }
}
