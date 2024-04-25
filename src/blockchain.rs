use crate::block::Block;
use serde::{Serialize, Deserialize};

pub struct BlockchainError {
    pub message: Option<String>,
}

impl BlockchainError {
    fn new(message: &str) -> BlockchainError {
        return BlockchainError{ message: Some(message.to_string()) };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blockchain {
    pub list: Vec<Block>,
}

impl Blockchain {
    pub fn new(genesis: &Block) -> Blockchain {
        Blockchain{
            list: vec![genesis.clone()],
        }
    }

    pub fn height(&self) -> usize{
        return self.list.len();
    }

    pub fn append(&mut self, block: &Block) -> Result<usize, BlockchainError> {
        if block.data.prev_hash == self.list[self.list.len() - 1].hash {
            self.list.push(block.clone());
            return Ok(self.list.len());
        }
        Err(BlockchainError::new("Cannot add block to chain"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyPair;
    use crate::hash::Hash;
    use crate::transaction::{Transaction, TransactionData, InPoint, OutPoint};
    use crate::block::{Block, BlockData};

    #[test]
    fn add_block() {
        let key = KeyPair::new();

        let block_0 = Block::new(
            &BlockData::new(&Hash::new(b"test"), 0, vec![
                Transaction::new(&TransactionData {
                    inputs: vec!(InPoint{ hash: Hash::new(b"test_1"), index: 0, signature: key.sign(b"test_1")}),
                    outputs: vec!(OutPoint{value: 1, pubkey: key.public_key() })
                })
            ]));

        let block_1 = Block::new(
            &BlockData::new(&block_0.hash, 0, vec![
                Transaction::new(&TransactionData {
                    inputs: vec!(InPoint{ hash: Hash::new(b"test_1"), index: 0, signature: key.sign(b"test_1")}),
                    outputs: vec!(OutPoint{value: 1, pubkey: key.public_key() })
                })
            ]));

        let mut chain = Blockchain::new(&block_0);
        assert_eq!(chain.height(), 1);

        let result = chain.append(&block_1);
        assert_eq!(chain.height(), 2);
        assert!(result.is_ok());
    }

    #[test]
    fn add_block_error() {
        let key = KeyPair::new();

        let block_0 = Block::new(
            &BlockData::new(&Hash::new(b"test_0"), 0, vec![
                Transaction::new(&TransactionData {
                    inputs: vec!(InPoint{ hash: Hash::new(b"test_1"), index: 0, signature: key.sign(b"test_1")}),
                    outputs: vec!(OutPoint{value: 1, pubkey: key.public_key() })
                })
            ]));

        let block_1 = Block::new(
            &BlockData::new(&Hash::new(b"test_1"), 0, vec![
                Transaction::new(&TransactionData {
                    inputs: vec!(InPoint{ hash: Hash::new(b"test_1"), index: 0, signature: key.sign(b"test_1")}),
                    outputs: vec!(OutPoint{value: 1, pubkey: key.public_key() })
                })
            ]));

        let mut chain = Blockchain::new(&block_0);
        assert_eq!(chain.height(), 1);

        let result = chain.append(&block_1);
        assert_eq!(chain.height(), 1);
        assert!(result.is_err());
    }
}
