use crate::block::Block;
use crate::hash::Hash;
use crate::io::{ByteIO, FileIO};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct BlockchainError {
    pub message: Option<String>,
}

impl BlockchainError {
    fn new(message: &str) -> BlockchainError {
        return BlockchainError {
            message: Some(message.to_string()),
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blockchain {
    pub list: Vec<Block>,
}

impl Blockchain {
    pub fn new(genesis: &Block) -> Blockchain {
        Blockchain {
            list: vec![genesis.clone()],
        }
    }

    pub fn height(&self) -> usize {
        return self.list.len();
    }

    pub fn append(&mut self, block: &Block) -> Result<usize, BlockchainError> {
        if block.data.prev_hash == self.list[self.list.len() - 1].hash {
            self.list.push(block.clone());
            return Ok(self.list.len() - 1);
        }
        Err(BlockchainError::new("Cannot add block to chain"))
    }

    pub fn query(&self, hash: &Hash) -> Option<(usize, &Block)> {
        for (i, block) in self.list.iter().enumerate().rev() {
            if block.hash == *hash {
                return Some((i, &block));
            }
        }
        return None;
    }
}

impl ByteIO for Blockchain {}

impl FileIO for Blockchain {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{Block, BlockData};
    use crate::keys::KeyPair;
    use crate::transaction::{InPoint, OutPoint, Transaction, TransactionData};

    struct BlockGen {
        valid: bool,
        keys: KeyPair,
        index: usize,
        prev_hash: Hash,
    }

    impl BlockGen {
        fn new(valid: bool) -> BlockGen {
            BlockGen {
                valid: valid,
                keys: KeyPair::new(),
                index: 0,
                prev_hash: Hash::new(b"genesis"),
            }
        }
    }

    impl Iterator for BlockGen {
        type Item = Block;

        fn next(&mut self) -> Option<Self::Item> {
            let name = format!("block-{}", self.index).into_bytes();
            let prev_hash = if self.valid {
                self.prev_hash.clone()
            } else {
                Hash::new(&name)
            };
            let block = Block::new(&BlockData::new(
                &prev_hash,
                0,
                vec![Transaction::new(TransactionData {
                    inputs: vec![InPoint {
                        hash: Hash::new(&name),
                        index: 0,
                        signature: self.keys.sign(&name),
                    }],
                    outputs: vec![OutPoint {
                        value: 1,
                        pubkey: self.keys.public_key(),
                    }],
                })],
            ));
            self.index += 1;
            self.prev_hash = block.hash.clone();
            Some(block)
        }
    }

    #[test]
    fn add_block() {
        let mut block_gen = BlockGen::new(true);

        let mut chain = Blockchain::new(&block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);

        let result = chain.append(&block_gen.next().unwrap());
        assert_eq!(chain.height(), 2);
        assert!(result.is_ok());
    }

    #[test]
    fn add_block_error() {
        let mut block_gen = BlockGen::new(false);

        let mut chain = Blockchain::new(&block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);

        let result = chain.append(&block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);
        assert!(result.is_err());
    }

    #[test]
    fn add_block_height() {
        let mut block_gen = BlockGen::new(true);

        let mut chain = Blockchain::new(&block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);

        let index = chain.append(&block_gen.next().unwrap()).unwrap();
        assert_eq!(chain.height(), 2);
        assert_eq!(index, 1);

        let index = chain.append(&block_gen.next().unwrap()).unwrap();
        assert_eq!(chain.height(), 3);
        assert_eq!(index, 2);
    }

    #[test]
    fn querying() {
        let mut block_gen = BlockGen::new(true);

        let mut chain = Blockchain::new(&block_gen.next().unwrap());
        let mut hashes = Vec::<(Hash, usize)>::new();
        for _ in 0..9 {
            let block = block_gen.next().unwrap();
            let index = chain.append(&block).unwrap();
            hashes.push((block.hash.clone(), index));
        }

        for (hash, index) in hashes {
            let (height, _) = chain.query(&hash).unwrap();
            assert_eq!(height, index);
        }

        let result = chain.query(&Hash::new(b"nothing"));
        assert!(result.is_none());
    }
}
