use crate::chain::Chain;
use crate::consensus::Target;
use crate::core::block::Block;
use crate::core::blockchain::TransactionValue;
use crate::core::hash::Hash;
use crate::core::keys::PublicKey;
use crate::core::transaction::{Output, Transaction, TransactionData};
use crate::utils::new_block;
use crate::utxo::Utxo;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum MiningError {
    NotEnoughTransactions,
    NoBlockFound,
}

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

    pub fn mine(&mut self, chain: &Chain) -> Result<Block, MiningError> {
        println!("Start mining");
        let tx_count: usize = 5;

        let mut rng = &mut rand::thread_rng();

        let mut txs = Vec::<Transaction>::new();
        let mut selected_utxos = HashSet::<Utxo>::new();
        for _ in 0..10 {
            txs.clear();
            selected_utxos.clear();
            for (_, tx) in self
                .pool
                .iter()
                .collect::<Vec<_>>()
                .choose_multiple(&mut rng, tx_count)
            {
                if merge_utxos(&tx, &mut selected_utxos) {
                    txs.push((*tx).clone());
                }
            }

            if txs.len() > 1 {
                break;
            }
        }

        if txs.is_empty() {
            println!("Failed to collect transactions");
            return Err(MiningError::NotEnoughTransactions);
        }

        let mut tx_value = TransactionValue::default();
        for tx in txs.iter() {
            self.pool.remove(&tx.hash);
            tx_value = tx_value + chain.chain.get_tx_value(tx).unwrap();
        }

        let coinbase_value = chain.rules.reward(chain.height()) + tx_value.fees;
        if coinbase_value > 0 {
            txs.push(Transaction::new(TransactionData::new_with_timestamp(
                vec![],
                vec![Output {
                    value: coinbase_value,
                    pubkey: self.recipient.clone(),
                }],
                chain.height() - 1,
            )));
        }

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
                self.cleanup_pool(&selected_utxos);
                return Ok(block);
            }
            let mut block_data = block.data;
            if block_data.nonce == u32::MAX {
                // Mining failed, reinsert transactions in pool
                for tx in block_data.transactions {
                    self.add_tx(chain, tx);
                }
                return Err(MiningError::NoBlockFound);
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

    pub fn cleanup_pool(&mut self, utxos: &HashSet<Utxo>) {
        self.pool.retain(|_, tx| utxos.is_disjoint(&get_utxos(&tx)))
    }
}

fn get_utxos(tx: &Transaction) -> HashSet<Utxo> {
    tx.data
        .inputs
        .iter()
        .map(|input| Utxo::new(input.hash.clone(), input.index, 0))
        .collect()
}

fn merge_utxos(tx: &Transaction, utxos: &mut HashSet<Utxo>) -> bool {
    let tx_utxos = get_utxos(tx);
    if !utxos.is_disjoint(&tx_utxos) {
        return false;
    }
    utxos.extend(tx_utxos.into_iter());
    return true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Chain;
    use crate::consensus::{ConsensusRules, Halving};
    use crate::core::keys::KeyPair;
    use crate::core::transaction::{Input, Output, TransactionData};

    #[test]
    fn mining() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let mut chain = Chain::new_with_consensus(
            &key_1.public_key(),
            ConsensusRules::new(Target::from_leading_zeros(0), 10000, Halving::None),
        );

        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];

        let tx = Transaction::new(TransactionData::new(
            vec![Input {
                hash: last_coinbase.hash.clone(),
                index: 0,
                signature: key_1.sign(last_coinbase.hash.digest()),
            }],
            vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        ));

        let mut miner = Miner::new(key_1.public_key());
        miner.add_tx(&chain, tx);

        assert_eq!(miner.pool.len(), 1);

        let block = miner.mine(&chain);

        assert!(block.is_ok());

        let block = block.unwrap();
        assert!(chain.validate_block(&block));

        let result = chain.add_block(block);

        assert!(result.is_ok());
    }
}
