use crate::errors::DeserializeError;
use crate::io::{FileIO, IntoBytes};
use ed25519_dalek::{Signature as DalekSignature, Signer, SigningKey, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

pub type PrivateKey = [u8; SECRET_KEY_LENGTH];
pub type PublicKey = [u8; SECRET_KEY_LENGTH];
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct KeyPair(SigningKey);

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Signature(DalekSignature);

impl KeyPair {
    pub fn new() -> KeyPair {
        let mut csprng = OsRng;
        KeyPair(SigningKey::generate(&mut csprng))
    }

    pub fn private_key(&self) -> PrivateKey {
        self.0.as_bytes().clone()
    }

    pub fn public_key(&self) -> PublicKey {
        self.0.verifying_key().to_bytes()
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        Signature(self.0.sign(message))
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        match self.0.verify(message, &signature.0) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

impl TryFrom<&[u8]> for KeyPair {
    type Error = DeserializeError;

    fn try_from(bytes: &[u8]) -> Result<KeyPair, DeserializeError> {
        if bytes.len() != 32 {
            return Err(DeserializeError);
        }
        let result: Result<&[u8; 32], core::array::TryFromSliceError> = bytes.try_into();
        match result {
            Ok(slice) => Ok(KeyPair(SigningKey::from_bytes(slice))),
            Err(_) => Err(DeserializeError),
        }
    }
}

impl IntoBytes for KeyPair {
    fn into_bytes(&self) -> Vec<u8> {
        Vec::from(self.private_key())
    }
}

impl FileIO for KeyPair {}

#[cfg(test)]
mod tests {
    use super::*;
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
        let pair = KeyPair::try_from(bytes.as_slice()).unwrap();
        let serialized = pair.into_bytes();
        assert_eq!(serialized, bytes);
    }

    #[test]
    fn deserialize() {
        let bytes = vec![0u8; 32];
        assert!(match KeyPair::try_from(bytes.as_slice()) {
            Ok(_) => true,
            Err(_) => false,
        });

        let bytes = vec![0u8; 31];
        assert!(match KeyPair::try_from(bytes.as_slice()) {
            Ok(_) => false,
            Err(_) => true,
        });

        let bytes = vec![0u8; 34];
        assert!(match KeyPair::try_from(bytes.as_slice()) {
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
}
