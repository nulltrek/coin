use crate::traits::io::{ByteIO, DeserializeError, FileIO};
use ed25519_dalek::{
    Signature as DalekSignature, Signer, SigningKey, Verifier as DalekVerifier, VerifyingKey,
    SECRET_KEY_LENGTH,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt;

pub trait Verifier {
    fn verify(&self, message: &[u8], signature: &Signature) -> bool;
}

pub type PrivateKey = [u8; SECRET_KEY_LENGTH];

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct PublicKey {
    value: [u8; SECRET_KEY_LENGTH],
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
        write!(
            f,
            "{}",
            self.value
                .iter()
                .map(|e| format!("{:02x}", e))
                .fold(String::new(), |mut acc, e| {
                    acc.push_str(&e);
                    acc
                })
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Signature(DalekSignature);

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
    use crate::types::hash::Hash;
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
    fn file_io() {
        let original = KeyPair::new();

        let temp_file = NamedTempFile::new().unwrap();

        let mut out_file = temp_file.reopen().unwrap();
        let result = original.to_file(&mut out_file);
        assert!(result.is_ok());

        let mut in_file = temp_file.reopen().unwrap();
        let deserialized = KeyPair::from_file(&mut in_file).unwrap();
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
