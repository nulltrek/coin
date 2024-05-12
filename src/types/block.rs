use crate::traits::io::{ByteIO, FileIO};
use crate::types::hash::Hash;
use crate::types::transaction::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct BlockData {
    pub prev_hash: Hash,
    pub nonce: u32,
    pub top_hash: Hash,
    pub transactions: Vec<Transaction>,
}

pub fn compute_top_hash(transactions: &[Transaction]) -> Hash {
    let digest_list: Vec<Vec<u8>> = transactions
        .iter()
        .map(|val| val.hash.digest().to_vec())
        .collect();
    Hash::new(&digest_list.concat().as_slice())
}

impl BlockData {
    pub fn new(prev_hash: Hash, nonce: u32, transactions: Vec<Transaction>) -> BlockData {
        BlockData {
            prev_hash,
            nonce,
            top_hash: compute_top_hash(&transactions),
            transactions,
        }
    }
}

impl ByteIO for BlockData {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Block {
    pub hash: Hash,
    pub data: BlockData,
}

impl Block {
    pub fn new(block_data: BlockData) -> Block {
        let bytes: Vec<u8> = bincode::serialize(&block_data).unwrap();
        Block {
            hash: Hash::new(bytes.as_slice()),
            data: block_data,
        }
    }

    pub fn is_hash_valid(&self) -> bool {
        let bytes: Vec<u8> = self.data.into_bytes();
        return Hash::new(bytes.as_slice()).digest() == self.hash.digest();
    }

    pub fn is_top_hash_valid(&self) -> bool {
        compute_top_hash(&self.data.transactions) == self.data.top_hash
    }
}

impl ByteIO for Block {}

impl FileIO for Block {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::keys::KeyPair;
    use crate::types::transaction::{Input, Output, TransactionData};
    use tempfile::*;

    #[test]
    fn hashing_equality() {
        let key = KeyPair::new();
        let txs = vec![
            Transaction::new(TransactionData {
                inputs: vec![Input {
                    hash: Hash::new(b"test_1"),
                    index: 0,
                    signature: key.sign(b"test_1"),
                }],
                outputs: vec![Output {
                    value: 1,
                    pubkey: key.public_key(),
                }],
            }),
            Transaction::new(TransactionData {
                inputs: vec![Input {
                    hash: Hash::new(b"test_2"),
                    index: 0,
                    signature: key.sign(b"test_2"),
                }],
                outputs: vec![Output {
                    value: 1,
                    pubkey: key.public_key(),
                }],
            }),
        ];

        let top_hash = Hash::new(
            vec![txs[0].hash.digest().to_vec(), txs[1].hash.digest().to_vec()]
                .concat()
                .as_slice(),
        );
        let block_data = BlockData::new(Hash::new(b"test"), 0, txs);
        assert_eq!(block_data.top_hash, top_hash)
    }

    #[test]
    fn hashing_inequality() {
        let key = KeyPair::new();

        let tx_1 = Transaction::new(TransactionData {
            inputs: vec![Input {
                hash: Hash::new(b"test_1"),
                index: 0,
                signature: key.sign(b"test_1"),
            }],
            outputs: vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        });
        let tx_2 = Transaction::new(TransactionData {
            inputs: vec![Input {
                hash: Hash::new(b"test_2"),
                index: 0,
                signature: key.sign(b"test_2"),
            }],
            outputs: vec![Output {
                value: 1,
                pubkey: key.public_key(),
            }],
        });

        let txs_1 = vec![tx_1.clone(), tx_2.clone()];
        let block_data_1 = BlockData::new(Hash::new(b"test"), 0, txs_1);

        let txs_2 = vec![tx_2.clone(), tx_1.clone()];
        let block_data_2 = BlockData::new(Hash::new(b"test"), 0, txs_2);

        assert_ne!(block_data_1.top_hash, block_data_2.top_hash)
    }

    #[test]
    fn file_io() {
        let key = KeyPair::new();

        let original = Block::new(BlockData::new(
            Hash::new(b"test"),
            0,
            vec![Transaction::new(TransactionData {
                inputs: vec![Input {
                    hash: Hash::new(b"test_1"),
                    index: 0,
                    signature: key.sign(b"test_1"),
                }],
                outputs: vec![Output {
                    value: 1,
                    pubkey: key.public_key(),
                }],
            })],
        ));

        let temp_file = NamedTempFile::new().unwrap();

        let mut out_file = temp_file.reopen().unwrap();
        let result = original.to_file(&mut out_file);
        assert!(result.is_ok());

        let mut in_file = temp_file.reopen().unwrap();
        let deserialized = Block::from_file(&mut in_file).unwrap();
        assert_eq!(original, deserialized);
    }
}
