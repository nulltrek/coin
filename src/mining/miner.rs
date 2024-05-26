use crate::chain::Chain;
use crate::consensus::Target;
use crate::core::block::Block;
use crate::core::blockchain::TransactionValue;
use crate::core::hash::Hash;
use crate::core::keys::PublicKey;
use crate::core::transaction::{Output, Transaction, TransactionData};
use crate::utils::new_block;
use std::collections::HashMap;

pub struct Miner {
    recipient: PublicKey,
    pub pool: HashMap<Hash, Transaction>,
}

impl Miner {
    pub fn new(recipient: PublicKey) -> Miner {
        Miner {
            recipient,
            pool: HashMap::new(),
        }
    }

    pub fn mine(&mut self, chain: &Chain) -> Option<Block> {
        let tx_count: usize = 10;
        let mut txs: Vec<Transaction> = self
            .pool
            .iter()
            .take(tx_count)
            .map(|(_, tx)| tx.clone())
            .collect();

        let mut tx_value = TransactionValue::default();
        for tx in txs.iter() {
            self.pool.remove(&tx.hash);
            tx_value = tx_value + chain.chain.get_tx_value(tx).unwrap();
        }

        txs.push(Transaction::new(TransactionData {
            inputs: vec![],
            outputs: vec![Output {
                value: chain.rules.coins_per_block + tx_value.fees,
                pubkey: self.recipient.clone(),
            }],
        }));

        println!("Target: {:0256b}", chain.rules.target);
        println!("Target leading: {}", chain.rules.target.leading_zeros());
        let mut leading: u32 = 0;
        let mut block = new_block(chain, 0, txs);
        loop {
            let block_target = Target::from_hash(&block.hash).leading_zeros();
            if block_target > leading {
                leading = block_target;
                println!("Leading: {}", leading);
            }
            if chain.rules.validate_target(&block.hash) {
                println!("Total tries: {}", block.data.nonce + 1);
                println!("Hash: {:0256b}", Target::from_hash(&block.hash));
                return Some(block);
            }
            let mut block_data = block.data;
            if block_data.nonce == u32::MAX {
                // Mining failed, reinsert transactions in pool
                for tx in block_data.transactions {
                    self.add_tx(chain, tx);
                }
                return None;
            }
            block_data.nonce += 1;

            if block_data.nonce % 100000 == 0 {
                println!("Tries: {}", block_data.nonce)
            }

            block = Block::new(block_data);
        }
    }

    pub fn add_tx(&mut self, chain: &Chain, tx: Transaction) -> bool {
        if chain.validate_tx(&tx) {
            self.pool.insert(tx.hash.clone(), tx);
            return true;
        }
        return false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Chain;
    use crate::consensus::ConsensusRules;
    use crate::core::keys::KeyPair;
    use crate::core::transaction::{Input, Output, TransactionData};

    #[test]
    fn mining() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let mut chain = Chain::new_with_consensus(
            &key_1.public_key(),
            ConsensusRules::new(Target::from_leading_zeros(0)),
        );

        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];

        let tx = Transaction::new(TransactionData {
            inputs: vec![Input {
                hash: last_coinbase.hash.clone(),
                index: 0,
                signature: key_1.sign(last_coinbase.hash.digest()),
            }],
            outputs: vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        });

        let mut miner = Miner::new(key_1.public_key());
        miner.add_tx(&chain, tx);

        assert_eq!(miner.pool.len(), 1);

        let block = miner.mine(&chain);

        assert!(block.is_some());

        let block = block.unwrap();
        assert!(chain.validate_block(&block));

        let result = chain.add_block(block);

        assert!(result.is_ok());
    }
}
