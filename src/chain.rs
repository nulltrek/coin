use crate::consensus::ConsensusRules;
use crate::traits::io::{ByteIO, FileIO};
use crate::types::block::Block;
use crate::types::blockchain::Blockchain;
use crate::types::keys::PublicKey;
use crate::types::transaction::{Output, Transaction};
use crate::utils::*;
use crate::utxo::Utxo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub enum ChainOpError {
    TargetNotSatisfied,
    InvalidBlock,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chain {
    pub rules: ConsensusRules,
    pub chain: Blockchain,
}

impl Chain {
    pub fn new(pubkey: &PublicKey) -> Chain {
        let rules = ConsensusRules::default();
        let genesis = new_genesis_block(&rules, pubkey);
        Chain {
            rules,
            chain: Blockchain::new(genesis),
        }
    }

    pub fn get_block(&self, height: usize) -> Option<&Block> {
        self.chain.list.get(height)
    }

    /*
     * The genesis block is the first block of the blockchain.
     * It's valid if:
     * - The value of prev_hash is all zeroes
     * - It contains only 1 coinbase transaction
     * - The transaction has 0 input and at least 1 output
     * - The value of the tx outputs must be less or equal to the
     *   coins_per_block value
     */
    pub fn validate_genesis(&self) -> bool {
        let genesis = &self.chain.list[0];

        return genesis.data.prev_hash.is_zero()
            && genesis.data.transactions.len() == 1
            && genesis.data.transactions[0].data.inputs.len() == 0
            && genesis.data.transactions[0].data.outputs.len() > 0
            && match get_block_value(&self.chain, &genesis) {
                Some(value) => value.output <= self.rules.coins_per_block,
                None => false,
            };
    }

    fn find_utxos<P>(&self, pred: P) -> Vec<Utxo>
    where
        P: Fn(&Output) -> bool,
    {
        let mut pool = HashMap::new();
        for block in self.chain.iter() {
            for tx in block.data.transactions.iter() {
                for (index, output) in tx.data.outputs.iter().enumerate() {
                    if pred(output) {
                        pool.insert((tx.hash.clone(), index), output.value);
                    }
                }

                for input in tx.data.inputs.iter() {
                    pool.remove(&(input.hash.clone(), input.index as usize));
                }
            }
        }
        pool.into_iter()
            .map(|(k, v)| Utxo::new(k.0, k.1 as u32, v))
            .collect()
    }

    pub fn find_all_utxos(&self) -> Vec<Utxo> {
        self.find_utxos(|_| true)
    }

    pub fn find_utxos_for_key(&self, pubkey: &PublicKey) -> Vec<Utxo> {
        self.find_utxos(|output| output.pubkey == *pubkey)
    }

    /*
     * A transaction is valid if:
     * - Its hash is valid
     * - There is at least 1 input
     * - There is at least 1 output
     * - For each input, its signature is valid (using the referenced output pubkey)
     * - For each output, its value is greater than zero
     * - The total input value is greater than or equal to the total ouput value
     */
    fn validate_tx(&self, tx: &Transaction) -> bool {
        return tx.is_hash_valid()
            && tx.data.inputs.len() > 0
            && tx.data.outputs.len() > 0
            && verify_tx_signatures(&self.chain, tx)
            && match get_tx_value(&self.chain, tx) {
                Some(value) => value.output > 0 && value.input >= value.output,
                None => false,
            };
    }
}

impl ByteIO for Chain {}
impl FileIO for Chain {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::keys::KeyPair;

    #[test]
    fn validate_genesis() {
        let key = KeyPair::new();
        let chain = Chain::new(&key.public_key());
        assert!(chain.validate_genesis());
    }

    #[test]
    fn find_utxos() {
        let key = KeyPair::new();
        let chain = Chain::new(&key.public_key());

        let utxos = chain.find_all_utxos();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].value, chain.rules.coins_per_block);

        let utxos = chain.find_utxos_for_key(&key.public_key());
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].value, chain.rules.coins_per_block);

        let utxos = chain.find_utxos_for_key(&KeyPair::new().public_key());
        assert_eq!(utxos.len(), 0);
    }

    // #[test]
    // fn validate_tx() {
    //     let key = KeyPair::new();
    //     let chain = Chain::new(&key.public_key());
    //     // TODO
    // }
}
