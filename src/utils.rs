use crate::consensus::ConsensusRules;
use crate::types::block::{Block, BlockData};
use crate::types::hash::Hash;
use crate::types::keys::PublicKey;
use crate::types::transaction::{Output, Transaction, TransactionData};

pub fn new_coinbase_tx(rules: &ConsensusRules, pubkey: &PublicKey) -> Transaction {
    Transaction::new(TransactionData {
        inputs: Vec::new(),
        outputs: vec![Output {
            value: rules.coins_per_block,
            pubkey: pubkey.clone(),
        }],
    })
}

pub fn new_genesis_block(rules: &ConsensusRules, pubkey: &PublicKey) -> Block {
    Block::new(BlockData::new(
        Hash::default(),
        0,
        vec![new_coinbase_tx(rules, pubkey)],
    ))
}

pub fn get_tx_value(tx: &Transaction) -> u64 {
    tx.data.outputs.iter().fold(0, |acc, o| acc + o.value)
}

pub fn get_block_value(block: &Block) -> u64 {
    block
        .data
        .transactions
        .iter()
        .fold(0, |acc, tx| acc + get_tx_value(tx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::keys::KeyPair;
    use crate::types::transaction::Input;
    use crate::types::testing::BlockGen;

    #[test]
    fn tx_value() {
        let key = KeyPair::new();
        let cr = ConsensusRules::default();
        let tx = new_coinbase_tx(&cr, &key.public_key());

        assert_eq!(get_tx_value(&tx), cr.coins_per_block);

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
        assert_eq!(get_tx_value(&tx), 77);
    }

    #[test]
    fn block_value() {
        let key = KeyPair::new();
        let cr = ConsensusRules::default();
        let block = new_genesis_block(&cr, &key.public_key());

        assert_eq!(get_block_value(&block), cr.coins_per_block);

        let mut block_gen = BlockGen::new(true, 10, 10);
        assert_eq!(get_block_value(&block_gen.next().unwrap()), 100);
    }
}
