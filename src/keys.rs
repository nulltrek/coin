use crate::errors::DeserializeError;
use ed25519_dalek::{Signature as DalekSignature, Signer, SigningKey, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;

pub type PrivateKey = [u8; SECRET_KEY_LENGTH];
pub type PublicKey = [u8; SECRET_KEY_LENGTH];

pub struct KeyPair(SigningKey);
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

    pub fn deserialize(key: Vec<u8>) -> Result<KeyPair, DeserializeError> {
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
}
