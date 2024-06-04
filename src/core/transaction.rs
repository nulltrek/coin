//! The means for exchanging coins between keys/addresses
//!
//! A transaction consumes all coins from a previous transaction and
//! assigns them to a new key/address. Coins are consumed from a previous
//! transaction and in turn can be spent in another transaction.
//!
//! The [inputs](Input) of a transactions represent the unspent coins from
//! a previous transaction's output. All the coins from the inputs will be consumed.
//!
//! Each of the [outputs](Output) of a transaction represent the number of coins to
//! be assigned to an address.
//!
//! Coinbase transactions are special transactions that generate new coins. A coinbase
//! transaction does not have inputs, only outputs.
//!

use crate::core::hash::Hash;
use crate::core::keys::{PublicKey, Signature};
use crate::traits::io::{ByteIO, FileIO, JsonIO};
use serde::{Deserialize, Serialize};

/// Utility type for representing coin value
pub type Value = u64;

/// The input identifies coins to be consumed.
///
/// An input points to a transaction through its [hash](struct@Hash), selects
/// an output of that transaction with its index, and provides a signature
/// for proving that who is spending the coins is the actual recipient of
/// the output.
///
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Input {
    pub hash: Hash,
    pub index: u32,
    pub signature: Signature,
}

/// An output specifies how many coins to be assigned to a [public key/address](PublicKey).
///
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Output {
    pub value: Value,
    pub pubkey: PublicKey,
}

/// The transaction data. It is composed by a list of inputs that will be consumed and
/// a list of outputs that will receive the consumed coins.
///
/// The timestamp is an optional field used only in coinbase transactions (i.e. transactions
/// which generate new coins).
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TransactionData {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub timestamp: Option<u64>,
}

impl TransactionData {
    pub fn new(inputs: Vec<Input>, outputs: Vec<Output>) -> TransactionData {
        TransactionData {
            inputs,
            outputs,
            timestamp: None,
        }
    }

    pub fn new_with_timestamp(
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        timestamp: u64,
    ) -> TransactionData {
        TransactionData {
            inputs,
            outputs,
            timestamp: Some(timestamp),
        }
    }
}

impl ByteIO for TransactionData {}

/// The Transaction struct is a wrapper for the [transaction data](TransactionData), it
/// computes and stores the hash of its contents.
///
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
        let bytes: Vec<u8> = self.data.into_bytes();
        return Hash::new(bytes.as_slice()).digest() == self.hash.digest();
    }

    pub fn is_coinbase(&self) -> bool {
        self.data.inputs.len() == 0
    }
}

impl ByteIO for Transaction {}
impl FileIO for Transaction {}
impl JsonIO for Transaction {}

impl ByteIO for Vec<Output> {}
impl FileIO for Vec<Output> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::keys::KeyPair;
    use tempfile::*;

    #[test]
    fn hashing_equality() {
        let key = KeyPair::new();
        let tx_data_1 = TransactionData::new(
            vec![Input {
                hash: Hash::new(b"test"),
                index: 0,
                signature: key.sign(b"test"),
            }],
            vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        );

        let tx_data_2 = TransactionData::new(
            vec![Input {
                hash: Hash::new(b"test"),
                index: 0,
                signature: key.sign(b"test"),
            }],
            vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        );

        let tx1 = Transaction::new(tx_data_1);
        let tx2 = Transaction::new(tx_data_2);

        assert_eq!(tx1.hash, tx2.hash)
    }

    #[test]
    fn is_coinbase() {
        let key = KeyPair::new();

        let tx = Transaction::new(TransactionData::new(
            vec![],
            vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        ));

        assert!(tx.is_coinbase());

        let tx = Transaction::new(TransactionData::new(
            vec![Input {
                hash: Hash::new(b"test"),
                index: 0,
                signature: key.sign(b"test"),
            }],
            vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        ));

        assert!(!tx.is_coinbase());
    }

    #[test]
    fn validation() {
        let key = KeyPair::new();
        let tx_data = TransactionData::new(
            vec![Input {
                hash: Hash::new(b"test"),
                index: 0,
                signature: key.sign(b"test"),
            }],
            vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        );

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
        let tx_data = TransactionData::new(
            vec![Input {
                hash: Hash::new(b"test"),
                index: 0,
                signature: key.sign(b"test"),
            }],
            vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        );

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
            data: TransactionData::new(
                vec![Input {
                    hash: Hash::new(b"test"),
                    index: 0,
                    signature: key.sign(b"test"),
                }],
                vec![Output {
                    value: 1,
                    pubkey: key.public_key(),
                }],
            ),
        };

        let temp_file = NamedTempFile::new().unwrap();

        let mut out_file = temp_file.reopen().unwrap();
        let result = original.to_file_descriptor(&mut out_file);
        assert!(result.is_ok());

        let mut in_file = temp_file.reopen().unwrap();
        let deserialized = Transaction::from_file_descriptor(&mut in_file).unwrap();
        assert_eq!(original, deserialized);
    }
}
