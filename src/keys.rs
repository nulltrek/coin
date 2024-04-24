use crate::errors::DeserializeError;
use ed25519_dalek::{Signature as DalekSignature, Signer, SigningKey, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;
use serde::{Serialize, Deserialize};

pub type PrivateKey = [u8; SECRET_KEY_LENGTH];
pub type PublicKey = [u8; SECRET_KEY_LENGTH];
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyPair(SigningKey);

#[derive(Serialize, Deserialize, Debug, Clone)]
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

    pub fn serialize(&self) -> Vec<u8> {
        self.private_key().to_vec()
    }

    pub fn deserialize(key: &Vec<u8>) -> Result<KeyPair, DeserializeError> {
        if key.len() != 32 {
            return Err(DeserializeError);
        }
        let result: Result<&[u8; 32], core::array::TryFromSliceError> =
            key.as_slice()[0..32].try_into();
        match result {
            Ok(slice) => Ok(KeyPair(SigningKey::from_bytes(slice))),
            Err(_) => Err(DeserializeError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing() {
        let key = KeyPair::new();
        let signature = key.sign(b"test");
        assert_eq!(key.verify(b"test", &signature), true);
    }

    #[test]
    fn serialize() {
        let bytes = vec![0u8; 32];
        let pair = KeyPair::deserialize(&bytes).unwrap();
        assert_eq!(pair.serialize(), bytes);
    }

    #[test]
    fn deserialize() {
        let bytes = vec![0u8; 32];
        assert!(match KeyPair::deserialize(&bytes) {
            Ok(_) => true,
            Err(_) => false,
        });

        let bytes = vec![0u8; 31];
        assert!(match KeyPair::deserialize(&bytes) {
            Ok(_) => false,
            Err(_) => true,
        });

        let bytes = vec![0u8; 34];
        assert!(match KeyPair::deserialize(&bytes) {
            Ok(_) => false,
            Err(_) => true,
        });
    }
}
