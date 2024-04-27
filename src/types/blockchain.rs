use crate::traits::io::{ByteIO, FileIO};
use crate::types::block::Block;
use crate::types::hash::Hash;
use crate::types::transaction::Transaction;
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
    pub fn new(genesis: Block) -> Blockchain {
        Blockchain {
            list: vec![genesis],
        }
    }

    pub fn height(&self) -> usize {
        return self.list.len();
    }

    pub fn append(&mut self, block: Block) -> Result<usize, BlockchainError> {
        if block.data.prev_hash == self.list[self.list.len() - 1].hash {
            self.list.push(block);
            return Ok(self.list.len() - 1);
        }
        Err(BlockchainError::new("Cannot add block to chain"))
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
}

impl ByteIO for Blockchain {}

impl FileIO for Blockchain {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::testing::BlockGen;

    #[test]
    fn add_block() {
        let mut block_gen = BlockGen::new(true);

        let mut chain = Blockchain::new(block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);

        let result = chain.append(block_gen.next().unwrap());
        assert_eq!(chain.height(), 2);
        assert!(result.is_ok());
    }

    #[test]
    fn add_block_error() {
        let mut block_gen = BlockGen::new(false);

        let mut chain = Blockchain::new(block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);

        let result = chain.append(block_gen.next().unwrap());
        assert_eq!(chain.height(), 1);
        assert!(result.is_err());
    }

    #[test]
    fn add_block_height() {
        let mut block_gen = BlockGen::new(true);

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
        let mut block_gen = BlockGen::new(true);

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
        let mut block_gen = BlockGen::new(true);

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
}
