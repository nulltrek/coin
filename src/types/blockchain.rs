use crate::traits::io::{ByteIO, FileIO};
use crate::types::block::Block;
use crate::types::hash::Hash;
use crate::types::transaction::{Output, Transaction, Value};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::slice::Iter;

#[derive(Debug)]
pub enum BlockchainError {
    InvalidPrevHash,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TransactionValue {
    pub input: Value,
    pub output: Value,
    pub fees: Value,
}

impl Default for TransactionValue {
    fn default() -> TransactionValue {
        TransactionValue {
            input: 0,
            output: 0,
            fees: 0,
        }
    }
}

impl TransactionValue {
    pub fn new(input: Value, output: Value, fees: Value) -> TransactionValue {
        TransactionValue {
            input,
            output,
            fees,
        }
    }
}

impl Add for TransactionValue {
    type Output = TransactionValue;

    fn add(self, rhs: Self) -> Self::Output {
        TransactionValue {
            input: self.input + rhs.input,
            output: self.output + rhs.output,
            fees: self.fees + rhs.fees,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blockchain {
    pub list: Vec<Block>,
}

impl Blockchain {
    pub fn new(genesis: Block) -> Blockchain {
        Blockchain {
            list: vec![genesis],
        }
    }

    pub fn height(&self) -> usize {
        return self.list.len();
    }

    pub fn iter(&self) -> Iter<'_, Block> {
        self.list.iter()
    }

    pub fn append(&mut self, block: Block) -> Result<usize, BlockchainError> {
        if block.data.prev_hash == self.list[self.list.len() - 1].hash {
            self.list.push(block);
            return Ok(self.list.len() - 1);
        }
        Err(BlockchainError::InvalidPrevHash)
    }

    pub fn get_block(&self, height: usize) -> Option<&Block> {
        if self.list.len() <= height {
            return None;
        }
        Some(&self.list[height])
    }

    pub fn get_last_block(&self) -> &Block {
        &self.list[self.list.len() - 1]
    }

    pub fn query_block(&self, hash: &Hash) -> Option<(usize, &Block)> {
        for (i, block) in self.list.iter().enumerate().rev() {
            if block.hash == *hash {
                return Some((i, &block));
            }
        }
        return None;
    }

    pub fn query_tx(&self, hash: &Hash) -> Option<(usize, &Transaction)> {
        for (i, block) in self.list.iter().enumerate().rev() {
            for tx in block.data.transactions.iter() {
                if tx.hash == *hash {
                    return Some((i, &tx));
                }
            }
        }
        return None;
    }

    pub fn get_tx_input_value(&self, tx: &Transaction) -> Option<Value> {
        let mut value: Value = 0;
        for input in &tx.data.inputs {
            let result = self.query_tx(&input.hash);
            if result.is_none() {
                return None;
            }
            let (_, tx) = result.unwrap();
            if tx.data.outputs.len() <= input.index as usize {
                return None;
            }
            value += tx.data.outputs[input.index as usize].value;
        }
        Some(value)
    }

    pub fn get_tx_output_value(outputs: &Vec<Output>) -> Value {
        outputs.iter().fold(0, |acc, o| acc + o.value)
    }

    pub fn get_tx_value(&self, tx: &Transaction) -> Option<TransactionValue> {
        let input: Value = match self.get_tx_input_value(tx) {
            Some(value) => value,
            None => return None,
        };
        let output = Self::get_tx_output_value(&tx.data.outputs);
        if input > 0 && output > input {
            return None;
        }
        let fees = if input == 0 { 0 } else { input - output };
        Some(TransactionValue::new(input, output, fees))
    }

    pub fn get_block_value(&self, block: &Block) -> Option<TransactionValue> {
        let mut acc = TransactionValue::default();
        for tx in block
            .data
            .transactions
            .iter()
            .filter(|tx| !tx.is_coinbase())
        {
            let result = self.get_tx_value(tx);
            if result.is_none() {
                return None;
            }
            acc = acc + result.unwrap();
        }
        Some(acc)
    }
}

impl ByteIO for Blockchain {}

impl FileIO for Blockchain {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::block::BlockData;
    use crate::types::keys::KeyPair;
    use crate::types::testing::BlockGen;
    use crate::types::transaction::{Input, TransactionData};
    use crate::utils::*;

    #[test]
    fn add_block() {
        let mut block_gen = BlockGen::default();

        let mut chain = Blockchain::new(block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);

        let result = chain.append(block_gen.next().unwrap());
        assert_eq!(chain.height(), 2);
        assert!(result.is_ok());
    }

    #[test]
    fn add_block_error() {
        let mut block_gen = BlockGen::new(false, 1, 1);

        let mut chain = Blockchain::new(block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);

        let result = chain.append(block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);
        assert!(result.is_err());
    }

    #[test]
    fn add_block_height() {
        let mut block_gen = BlockGen::default();

        let mut chain = Blockchain::new(block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);

        let index = chain.append(block_gen.next().unwrap()).unwrap();
        assert_eq!(chain.height(), 2);
        assert_eq!(index, 1);

        let index = chain.append(block_gen.next().unwrap()).unwrap();
        assert_eq!(chain.height(), 3);
        assert_eq!(index, 2);
    }

    #[test]
    fn query_blocks() {
        let mut block_gen = BlockGen::default();

        let mut chain = Blockchain::new(block_gen.next().unwrap());
        let mut hashes = Vec::<(Hash, usize)>::new();
        for _ in 0..9 {
            let block = block_gen.next().unwrap();
            let block_hash = block.hash.clone();
            let index = chain.append(block).unwrap();
            hashes.push((block_hash, index));
        }

        for (hash, index) in hashes {
            let (height, _) = chain.query_block(&hash).unwrap();
            assert_eq!(height, index);
        }

        let result = chain.query_block(&Hash::new(b"nothing"));
        assert!(result.is_none());
    }

    use rand::seq::SliceRandom;
    use rand::thread_rng;

    #[test]
    fn query_txs() {
        let mut rng = thread_rng();
        let mut block_gen = BlockGen::default();

        let mut chain = Blockchain::new(block_gen.next().unwrap());
        let mut hashes = Vec::<(Hash, usize)>::new();
        for _ in 0..9 {
            let block = block_gen.next().unwrap();

            let tx = block.data.transactions.choose(&mut rng).unwrap();
            let tx_hash = tx.hash.clone();
            let height = chain.append(block).unwrap();
            hashes.push((tx_hash, height));
        }

        for (hash, index) in hashes {
            let (height, _) = chain.query_tx(&hash).unwrap();
            assert_eq!(height, index);
        }

        let result = chain.query_tx(&Hash::new(b"nothing"));
        assert!(result.is_none());
    }

    #[test]
    fn tx_output_value() {
        let key = KeyPair::new();
        let tx = Transaction::new(TransactionData {
            inputs: Vec::new(),
            outputs: vec![Output {
                value: 10000,
                pubkey: key.public_key().clone(),
            }],
        });

        assert_eq!(Blockchain::get_tx_output_value(&tx.data.outputs), 10000);

        let tx = Transaction::new(TransactionData {
            inputs: vec![Input {
                hash: Hash::new(b"test"),
                index: 0,
                signature: key.sign(b"test"),
            }],
            outputs: vec![
                Output {
                    value: 10,
                    pubkey: key.public_key(),
                },
                Output {
                    value: 5,
                    pubkey: key.public_key(),
                },
                Output {
                    value: 62,
                    pubkey: key.public_key(),
                },
            ],
        });
        assert_eq!(Blockchain::get_tx_output_value(&tx.data.outputs), 77);
    }

    #[test]
    fn tx_get_value() {
        let coinbase_value: Value = 10000;
        let key = KeyPair::new();
        let chain = Blockchain::new(new_genesis_block(&key.public_key(), coinbase_value));
        let coinbase = &chain.list[0].data.transactions[0];

        let make_tx = |hash: &Hash, value: Value| {
            Transaction::new(TransactionData {
                inputs: vec![Input {
                    hash: hash.clone(),
                    index: 0,
                    signature: key.sign(hash.digest()),
                }],
                outputs: vec![Output {
                    value: value,
                    pubkey: key.public_key(),
                }],
            })
        };

        let tx = make_tx(&coinbase.hash, 5000);
        let value = chain.get_tx_value(&tx);
        assert!(value.is_some());
        assert_eq!(
            chain.get_tx_value(&tx).unwrap(),
            TransactionValue {
                input: coinbase_value,
                output: 5000,
                fees: coinbase_value - 5000,
            }
        );

        let tx = make_tx(&coinbase.hash, coinbase_value);
        let value = chain.get_tx_value(&tx);
        assert!(value.is_some());
        assert_eq!(
            chain.get_tx_value(&tx).unwrap(),
            TransactionValue {
                input: coinbase_value,
                output: coinbase_value,
                fees: 0,
            }
        );

        let tx = make_tx(&coinbase.hash, coinbase_value + 1);
        let value = chain.get_tx_value(&tx);
        assert!(value.is_none());
    }

    #[test]
    fn block_value() {
        let coinbase_value: Value = 10000;
        let key = KeyPair::new();

        let genesis = new_genesis_block(&key.public_key(), coinbase_value);
        let genesis_hash = genesis.hash.clone();
        let coinbase_hash = genesis.data.transactions[0].hash.clone();

        let chain = Blockchain::new(genesis);

        let next = Block::new(BlockData::new(
            genesis_hash,
            0,
            vec![
                Transaction::new(TransactionData {
                    inputs: vec![Input {
                        hash: coinbase_hash.clone(),
                        index: 0,
                        signature: key.sign(coinbase_hash.digest()),
                    }],
                    outputs: vec![
                        Output {
                            value: 5000,
                            pubkey: key.public_key(),
                        },
                        Output {
                            value: 4000,
                            pubkey: key.public_key(),
                        },
                    ],
                }),
                new_coinbase_tx(&key.public_key(), coinbase_value),
            ],
        ));

        let value = chain.get_block_value(&next).unwrap();

        assert_eq!(value.input, coinbase_value);
        assert_eq!(value.output, 9000);
        assert_eq!(value.fees, 1000);
    }
}
