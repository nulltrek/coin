use crate::types::hash::Hash;
use crate::types::block::{Block, BlockData};
use crate::types::keys::KeyPair;
use crate::types::transaction::{InPoint, OutPoint, Transaction, TransactionData};

pub struct BlockGen {
    valid: bool,
    keys: KeyPair,
    index: usize,
    prev_hash: Hash,
}

impl BlockGen {
    pub fn new(valid: bool) -> BlockGen {
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
        let block = Block::new(BlockData::new(
            prev_hash,
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
