use crate::traits::io::{ByteIO, FileIO};
use crate::types::hash::Hash;
use crate::types::keys::{PublicKey, Signature};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Input {
    pub hash: Hash,
    pub index: u32,
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Output {
    pub value: u64,
    pub pubkey: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TransactionData {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {
    pub hash: Hash,
    pub data: TransactionData,
}

impl Transaction {
    pub fn new(tx_data: TransactionData) -> Transaction {
        let bytes: Vec<u8> = bincode::serialize(&tx_data).unwrap();
        Transaction {
            hash: Hash::new(bytes.as_slice()),
            data: tx_data,
        }
    }

    pub fn is_hash_valid(&self) -> bool {
        let bytes: Vec<u8> = bincode::serialize(&self.data).unwrap();
        return Hash::new(bytes.as_slice()).digest() == self.hash.digest();
    }
}

impl ByteIO for Transaction {}

impl FileIO for Transaction {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::keys::KeyPair;
    use tempfile::*;

    #[test]
    fn hashing_equality() {
        let key = KeyPair::new();
        let tx_data_1 = TransactionData {
            inputs: vec![Input {
                hash: Hash::new(b"test"),
                index: 0,
                signature: key.sign(b"test"),
            }],
            outputs: vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        };

        let tx_data_2 = TransactionData {
            inputs: vec![Input {
                hash: Hash::new(b"test"),
                index: 0,
                signature: key.sign(b"test"),
            }],
            outputs: vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        };

        let tx1 = Transaction::new(tx_data_1);
        let tx2 = Transaction::new(tx_data_2);

        assert_eq!(tx1.hash, tx2.hash)
    }

    #[test]
    fn validation() {
        let key = KeyPair::new();
        let tx_data = TransactionData {
            inputs: vec![Input {
                hash: Hash::new(b"test"),
                index: 0,
                signature: key.sign(b"test"),
            }],
            outputs: vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        };

        let tx_1 = Transaction::new(tx_data.clone());
        assert!(tx_1.is_hash_valid());

        let tx_2 = Transaction {
            hash: Hash::new(b"test"),
            data: tx_data,
        };
        assert!(!tx_2.is_hash_valid());
    }

    #[test]
    fn unserialize_validation() {
        let key = KeyPair::new();
        let tx_data = TransactionData {
            inputs: vec![Input {
                hash: Hash::new(b"test"),
                index: 0,
                signature: key.sign(b"test"),
            }],
            outputs: vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        };

        let tx = Transaction {
            hash: Hash::new(b"test"),
            data: tx_data,
        };

        let bytes = tx.into_bytes();

        let deserialized_tx = Transaction::from_bytes(bytes.as_slice()).unwrap();
        assert!(!deserialized_tx.is_hash_valid());
    }

    #[test]
    fn file_io() {
        let key = KeyPair::new();

        let original = Transaction {
            hash: Hash::new(b"test"),
            data: TransactionData {
                inputs: vec![Input {
                    hash: Hash::new(b"test"),
                    index: 0,
                    signature: key.sign(b"test"),
                }],
                outputs: vec![Output {
                    value: 1,
                    pubkey: key.public_key(),
                }],
            },
        };

        let temp_file = NamedTempFile::new().unwrap();

        let mut out_file = temp_file.reopen().unwrap();
        let result = original.to_file(&mut out_file);
        assert!(result.is_ok());

        let mut in_file = temp_file.reopen().unwrap();
        let deserialized = Transaction::from_file(&mut in_file).unwrap();
        assert_eq!(original, deserialized);
    }
}
