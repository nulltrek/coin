use crate::core::block::{Block, BlockData};
use crate::core::hash::Hash;
use crate::core::keys::KeyPair;
use crate::core::transaction::{Input, Output, Transaction, TransactionData, Value};

pub struct BlockGen {
    valid: bool,
    keys: KeyPair,
    index: usize,
    prev_hash: Hash,
    pub output_count: u32,
    output_value: Value,
}

impl Default for BlockGen {
    fn default() -> BlockGen {
        BlockGen {
            valid: true,
            keys: KeyPair::new(),
            index: 0,
            prev_hash: Hash::new(b"genesis"),
            output_count: 1,
            output_value: 1,
        }
    }
}

impl BlockGen {
    pub fn new(valid: bool, output_count: u32, output_value: Value) -> BlockGen {
        BlockGen {
            valid,
            output_count,
            output_value,
            ..BlockGen::default()
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
                inputs: vec![Input {
                    hash: Hash::new(&name),
                    index: 0,
                    signature: self.keys.sign(&name),
                }],
                outputs: (0..self.output_count)
                    .map(|_| Output {
                        value: self.output_value,
                        pubkey: self.keys.public_key(),
                    })
                    .collect(),
            })],
        ));
        self.index += 1;
        self.prev_hash = block.hash.clone();
        Some(block)
    }
}
