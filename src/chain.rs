//! A blockchain implementation that follow consensus rules and validates blocks and transactions
//!
//! This chain builds on the base blockchain struct and implements all the validation functions
//! for making sure no invalid transactions or blocks get added to the list.
//!

use crate::consensus::ConsensusRules;
use crate::core::block::Block;
use crate::core::blockchain::{Blockchain, BlockchainError, Height};
use crate::core::hash::Hash;
use crate::core::keys::{PublicKey, Verifier};
use crate::core::transaction::{Output, Transaction};
use crate::traits::io::{ByteIO, FileIO, JsonIO};
use crate::utils::*;
use crate::utxo::Utxo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;

/// A pool of UTXOs that can be used to track the current unspent outputs.
/// Provides utility functions for querying the current state of coins that
/// can be exchanged between addresses.
///
#[derive(Debug, Clone)]
pub struct UtxoPool {
    pub utxos: HashMap<(Hash, u32), Output>,
}

impl Default for UtxoPool {
    fn default() -> UtxoPool {
        UtxoPool {
            utxos: HashMap::new(),
        }
    }
}

impl UtxoPool {
    pub fn new(chain: &Blockchain) -> UtxoPool {
        let mut pool = UtxoPool {
            utxos: HashMap::new(),
        };
        for block in chain.iter() {
            pool.update(block);
        }
        pool
    }

    pub fn get_with_pred<P>(&self, pred: P) -> Vec<Utxo>
    where
        P: Fn(&Output) -> bool,
    {
        self.utxos
            .iter()
            .filter(|(_, output)| pred(output))
            .map(|(k, v)| Utxo::new(k.0.clone(), k.1, v.value))
            .collect()
    }

    pub fn get_all(&self) -> Vec<Utxo> {
        self.get_with_pred(|_| true)
    }

    pub fn get_for_key(&self, pubkey: &PublicKey) -> Vec<Utxo> {
        self.get_with_pred(|output| output.pubkey == *pubkey)
    }

    pub fn update(&mut self, block: &Block) {
        for tx in block.data.transactions.iter() {
            for (index, output) in tx.data.outputs.iter().enumerate() {
                self.utxos
                    .insert((tx.hash.clone(), index as u32), output.clone());
            }

            for input in tx.data.inputs.iter() {
                self.utxos.remove(&(input.hash.clone(), input.index));
            }
        }
    }

    pub fn is_unspent(&self, tx: &Transaction) -> bool {
        for input in &tx.data.inputs {
            // TODO: avoid cloning
            if !self.utxos.contains_key(&(input.hash.clone(), input.index)) {
                return false;
            }
        }
        return true;
    }
}

/// Enum for specifying different possible chain operation errors
///
#[derive(PartialEq, Debug)]
pub enum ChainOpError {
    InvalidChain,
    TargetNotSatisfied,
    InvalidBlock,
    InvalidPrevHash,
}

/// A consensus-following, full validating blockchain.
///
/// Every block that someone tries to add to the chain is verified for correctness
/// from the point of view of the consensus rules and the cryptographic soundness.
///
/// The chain maintains a pool of current unspent UTXOs for faster validation.
///
#[derive(Debug, Clone)]
pub struct Chain {
    pub rules: ConsensusRules,
    pub chain: Blockchain,
    utxos: UtxoPool,
}

impl Chain {
    fn init(rules: ConsensusRules, chain: Blockchain) -> Chain {
        let utxos = UtxoPool::new(&chain);
        Chain {
            rules,
            chain,
            utxos,
        }
    }

    pub fn new(pubkey: &PublicKey) -> Chain {
        let rules = ConsensusRules::default();
        let genesis = new_genesis_block(pubkey, rules.base_coins);
        Self::init(rules, Blockchain::new(genesis))
    }

    pub fn new_with_consensus(pubkey: &PublicKey, rules: ConsensusRules) -> Chain {
        let genesis = new_genesis_block(pubkey, rules.base_coins);
        Self::init(rules, Blockchain::new(genesis))
    }

    pub fn from_serializable(chain: SerializableChain) -> Result<Chain, ChainOpError> {
        let chain = Self::init(chain.rules, chain.chain);
        if !chain.validate_chain() {
            return Err(ChainOpError::InvalidChain);
        }
        Ok(chain)
    }

    pub fn get_block(&self, height: usize) -> Option<&Block> {
        self.chain.list.get(height)
    }

    pub fn get_last_block(&self) -> &Block {
        self.chain.get_last_block()
    }

    pub fn height(&self) -> Height {
        self.chain.height()
    }

    pub fn find_all_utxos(&self) -> Vec<Utxo> {
        self.utxos.get_all()
    }

    pub fn find_utxos_for_key(&self, pubkey: &PublicKey) -> Vec<Utxo> {
        self.utxos.get_for_key(pubkey)
    }

    /// Verifies if the signatures provided in all the transaction inputs are actually
    /// associated to the public keys defined in the referenced outputs.
    ///
    fn verify_tx_signatures(&self, tx: &Transaction) -> bool {
        for input in &tx.data.inputs {
            let idx = input.index as usize;
            let (_, input_tx) = match self.chain.query_tx(&input.hash) {
                Some(result) => result,
                None => return false,
            };

            if input_tx.data.outputs.len() <= idx {
                return false;
            }

            if !input_tx.data.outputs[idx]
                .pubkey
                .verify(input_tx.hash.digest(), &input.signature)
            {
                return false;
            }
        }
        return true;
    }

    /// The genesis block is the first block of the blockchain.
    /// It's valid if:
    /// - The value of prev_hash is all zeroes
    /// - It contains only 1 coinbase transaction
    /// - The transaction has 0 input and at least 1 output
    /// - The value of the tx outputs must be less or equal to the
    ///   base_coins value
    ///
    fn validate_genesis(&self) -> bool {
        let genesis = &self.chain.list[0];
        return genesis.data.prev_hash.is_zero()
            && genesis.data.transactions.len() == 1
            && genesis.data.transactions[0].data.inputs.len() == 0
            && genesis.data.transactions[0].data.outputs.len() > 0
            && match self.chain.get_tx_value(&genesis.data.transactions[0]) {
                Some(value) => value.output <= self.rules.base_coins,
                None => false,
            };
    }

    /// A transaction is valid if:
    /// - Its hash is valid
    /// - There is at least 1 input
    /// - There is at least 1 output
    /// - For each input, its signature is valid (using the referenced output pubkey)
    /// - The inputs are unspent
    /// - For each output, its value is greater than zero
    /// - The total input value is greater than or equal to the total ouput value
    /// - It doesn't have a timestamp
    ///
    fn validate_tx(&self, tx: &Transaction, utxos: &UtxoPool) -> bool {
        return tx.is_hash_valid()
            && tx.data.inputs.len() > 0
            && tx.data.outputs.len() > 0
            && self.verify_tx_signatures(tx)
            && utxos.is_unspent(tx)
            && match self.chain.get_tx_value(tx) {
                Some(value) => value.output > 0 && value.input >= value.output,
                None => false,
            }
            && tx.data.timestamp.is_none();
    }

    /// Validates a transaction using the current UTXO pool as base
    ///
    pub fn validate_new_tx(&self, tx: &Transaction) -> bool {
        self.validate_tx(tx, &self.utxos)
    }

    /// A coinbase transaction is valid on a collection of transactions if:
    /// - Its hash is valid
    /// - There are no inputs
    /// - There is at least 1 output
    /// - The coinbase transaction timestamp must be equal to the provided block height
    /// - The total output value is less than or equal to the consensus reward + fees on the tx collection
    ///
    fn validate_coinbase_tx(
        &self,
        prev_block_hash: &Hash,
        txs: &[Transaction],
        tx: &Transaction,
    ) -> bool {
        let txs_value = self.chain.get_tx_collection_value(txs);
        if txs_value.is_none() {
            return false;
        }
        return tx.is_hash_valid()
            && tx.data.inputs.len() == 0
            && tx.data.outputs.len() > 0
            && (prev_block_hash.is_zero()
                || match self.chain.query_block(&prev_block_hash) {
                    Some((height, _)) => {
                        tx.data.timestamp.is_some() && tx.data.timestamp.unwrap() == height as u64
                    }
                    None => false,
                })
            && match self.chain.get_tx_value(tx) {
                Some(value) => {
                    let height = match tx.data.timestamp {
                        None => 0,
                        Some(value) => value + 1,
                    };
                    value.input == 0
                        && value.output > 0
                        && value.output
                            <= (self.rules.reward(Height::from(height)) + txs_value.unwrap().fees)
                }
                None => false,
            };
    }

    /// The order of transactions in a block matters. The same utxo cannot be
    /// spent twice in the same transaction and there cannot be two or more
    /// transactions in a block which spend the same utxo. This function checks
    /// for double spends in a list of transactions.
    ///
    fn validate_double_spend(&self, transactions: &[Transaction]) -> bool {
        let mut inputs = HashSet::<(Hash, u32)>::new();
        for tx in transactions {
            for input in &tx.data.inputs {
                let cur = (input.hash.clone(), input.index);
                if inputs.contains(&cur) {
                    return false;
                }
                inputs.insert(cur);
            }
        }
        return true;
    }

    /// A block is valid if:
    /// - Its hash is valid
    /// - The block points to the previous block
    /// - There is at least one transaction
    /// - If there is only one transaction, it's a regular transaction
    /// - The top hash is valid
    /// - All the transactions except the last one are valid regular transactions
    /// - The last transaction is a valid coinbase transaction or a valid regular transaction
    ///
    fn validate_block(&self, block: &Block, previous: &Block, utxos: &UtxoPool) -> bool {
        return block.is_hash_valid()
            && block.data.prev_hash == previous.hash
            && block.data.transactions.len() > 0
            && match block.data.transactions.len() {
                1 => !block.data.transactions.last().unwrap().is_coinbase(),
                _ => true,
            }
            && block.is_top_hash_valid()
            && block.data.transactions[..block.data.transactions.len() - 1]
                .iter()
                .fold(true, |acc, tx| acc && self.validate_tx(tx, utxos))
            && (self.validate_coinbase_tx(
                block.prev_hash(),
                block.transactions(),
                block.transactions().last().unwrap(),
            ) || self.validate_tx(block.transactions().last().unwrap(), utxos))
            && self.validate_double_spend(&block.data.transactions);
    }

    /// Validates a block using the current last block and UTXO pool as base
    ///
    pub fn validate_new_block(&self, block: &Block) -> bool {
        self.validate_block(block, self.chain.get_last_block(), &self.utxos)
    }

    /// A chain is valid if:
    /// - The genesis block is valid
    /// - All the remaining blocks are valid
    ///
    pub fn validate_chain(&self) -> bool {
        let mut utxos = UtxoPool::default();
        utxos.update(&self.chain.list[0]);
        return self.validate_genesis()
            && self.chain.list[1..]
                .iter()
                .enumerate()
                .fold(true, |acc, (i, block)| {
                    let result = acc && self.validate_block(block, &self.chain.list[i], &utxos);
                    utxos.update(block);
                    result
                });
    }

    /// A block can be added to the blockchain if:
    /// - Its hash satisfies the consensus target
    /// - It's a valid block
    ///
    pub fn add_block(&mut self, block: Block) -> Result<Height, ChainOpError> {
        if !self.rules.validate_target(&block.hash) {
            return Err(ChainOpError::TargetNotSatisfied);
        }
        if !self.validate_new_block(&block) {
            return Err(ChainOpError::InvalidBlock);
        }

        match self.chain.append(block) {
            Err(BlockchainError::InvalidPrevHash) => Err(ChainOpError::InvalidPrevHash),
            Ok(value) => {
                let block = self.get_last_block();
                self.utxos.update(&block.clone());
                Ok(value)
            }
        }
    }
}

/// Helper struct for serializing and deserializing a [chain](Chain)
///
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableChain {
    pub rules: ConsensusRules,
    pub chain: Blockchain,
}

impl SerializableChain {
    pub fn new(chain: Chain) -> SerializableChain {
        SerializableChain {
            rules: chain.rules,
            chain: chain.chain,
        }
    }
}

impl ByteIO for SerializableChain {}
impl FileIO for SerializableChain {}
impl JsonIO for SerializableChain {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::{Halving, Target};
    use crate::core::block::{Block, BlockData};
    use crate::core::hash::Hash;
    use crate::core::keys::KeyPair;
    use crate::core::transaction::{Input, Output, TransactionData, Value};
    use ethnum::U256;

    #[test]
    fn validate_genesis() {
        let key = KeyPair::new();
        let mut chain = Chain::new(&key.public_key());
        assert!(chain.validate_genesis());

        chain.chain.list[0].data.transactions[0].data.outputs[0].value -= 100;
        assert!(chain.validate_genesis());

        chain.chain.list[0].data.transactions[0].data.outputs[0].value += 101;
        assert!(!chain.validate_genesis());
    }

    #[test]
    fn find_utxos() {
        let key = KeyPair::new();
        let chain = Chain::new(&key.public_key());

        let utxos = chain.find_all_utxos();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].value, chain.rules.base_coins);

        let utxos = chain.find_utxos_for_key(&key.public_key());
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].value, chain.rules.base_coins);

        let utxos = chain.find_utxos_for_key(&KeyPair::new().public_key());
        assert_eq!(utxos.len(), 0);
    }

    #[test]
    fn unspent_utxos() {
        let key = KeyPair::new();
        let mut chain = Chain::new(&key.public_key());

        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];
        let last_block_hash = last_block.hash.clone();

        let tx = Transaction::new(TransactionData::new(
            vec![Input {
                hash: last_coinbase.hash.clone(),
                index: 0,
                signature: key.sign(last_coinbase.hash.digest()),
            }],
            vec![Output {
                value: 5000,
                pubkey: key.public_key(),
            }],
        ));

        assert!(chain.utxos.is_unspent(&tx));

        let result = chain.add_block(Block::new(BlockData::new(
            last_block_hash,
            0,
            vec![tx.clone()],
        )));

        assert!(result.is_ok());

        assert!(!chain.utxos.is_unspent(&tx));
    }

    #[test]
    fn signature_verification() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let chain = Chain::new(&key_1.public_key());
        let utxos = chain.find_utxos_for_key(&key_1.public_key());

        let tx = new_tx(
            &key_1,
            &utxos,
            vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        );

        assert!(chain.verify_tx_signatures(&tx.unwrap()));

        let tx = new_tx(
            &key_2,
            &utxos,
            vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        )
        .unwrap();

        assert!(!chain.verify_tx_signatures(&tx));
    }

    #[test]
    fn validate_new_tx() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();
        let chain = Chain::new(&key_1.public_key());
        let coinbase = &chain.chain.list[0].data.transactions[0];

        let tx = Transaction::new(TransactionData::new(
            vec![Input {
                hash: coinbase.hash.clone(),
                index: 0,
                signature: key_1.sign(coinbase.hash.digest()),
            }],
            vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        ));
        assert!(chain.validate_new_tx(&tx));

        let tx = Transaction::new(TransactionData::new(
            vec![],
            vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        ));
        assert!(!chain.validate_new_tx(&tx));

        let tx = Transaction::new(TransactionData::new(
            vec![Input {
                hash: coinbase.hash.clone(),
                index: 0,
                signature: key_1.sign(coinbase.hash.digest()),
            }],
            vec![],
        ));
        assert!(!chain.validate_new_tx(&tx));

        let tx = Transaction::new(TransactionData::new(
            vec![Input {
                hash: coinbase.hash.clone(),
                index: 0,
                signature: key_2.sign(coinbase.hash.digest()),
            }],
            vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        ));
        assert!(!chain.validate_new_tx(&tx));

        let tx = Transaction::new(TransactionData::new(
            vec![Input {
                hash: coinbase.hash.clone(),
                index: 0,
                signature: key_1.sign(coinbase.hash.digest()),
            }],
            vec![Output {
                value: 0,
                pubkey: key_2.public_key(),
            }],
        ));
        assert!(!chain.validate_new_tx(&tx));

        let tx = Transaction::new(TransactionData::new(
            vec![Input {
                hash: coinbase.hash.clone(),
                index: 0,
                signature: key_1.sign(coinbase.hash.digest()),
            }],
            vec![Output {
                value: chain.rules.base_coins + 1,
                pubkey: key_2.public_key(),
            }],
        ));
        assert!(!chain.validate_new_tx(&tx));
    }

    #[test]
    fn validate_coinbase_tx() {
        let key = KeyPair::new();
        let chain = Chain::new(&key.public_key());

        let genesis = &chain.chain.list[0];
        let coinbase = &genesis.data.transactions[0];
        assert!(chain.validate_coinbase_tx(genesis.prev_hash(), genesis.transactions(), &coinbase));

        let tx = Transaction {
            hash: Hash::new(b"test"),
            data: coinbase.data.clone(),
        };
        assert!(!chain.validate_coinbase_tx(genesis.prev_hash(), genesis.transactions(), &tx));

        let tx = Transaction::new(TransactionData::new(vec![], vec![]));
        assert!(!chain.validate_coinbase_tx(genesis.prev_hash(), genesis.transactions(), &tx));

        let tx = Transaction::new(TransactionData::new(
            vec![],
            vec![Output {
                value: 0,
                pubkey: key.public_key(),
            }],
        ));
        assert!(!chain.validate_coinbase_tx(genesis.prev_hash(), genesis.transactions(), &tx));

        let tx = Transaction::new(TransactionData::new(
            vec![],
            vec![Output {
                value: chain.rules.base_coins,
                pubkey: key.public_key(),
            }],
        ));
        assert!(chain.validate_coinbase_tx(genesis.prev_hash(), genesis.transactions(), &tx));

        let tx = Transaction::new(TransactionData::new_with_timestamp(
            vec![],
            vec![Output {
                value: chain.rules.base_coins + 5000,
                pubkey: key.public_key(),
            }],
            0,
        ));
        let block = Block::new(BlockData::new(
            genesis.hash.clone(),
            0,
            vec![
                Transaction::new(TransactionData::new(
                    vec![Input {
                        hash: coinbase.hash.clone(),
                        index: 0,
                        signature: key.sign(coinbase.hash.digest()),
                    }],
                    vec![Output {
                        value: 5000,
                        pubkey: key.public_key(),
                    }],
                )),
                tx.clone(),
            ],
        ));

        assert!(chain.validate_coinbase_tx(block.prev_hash(), block.transactions(), &tx));

        let tx = Transaction::new(TransactionData::new_with_timestamp(
            vec![],
            vec![Output {
                value: chain.rules.base_coins,
                pubkey: key.public_key(),
            }],
            0,
        ));

        assert!(chain.validate_coinbase_tx(block.prev_hash(), block.transactions(), &tx));

        let tx = Transaction::new(TransactionData::new_with_timestamp(
            vec![],
            vec![Output {
                value: chain.rules.base_coins + 5001,
                pubkey: key.public_key(),
            }],
            0,
        ));

        assert!(!chain.validate_coinbase_tx(block.prev_hash(), block.transactions(), &tx));
    }

    #[test]
    fn validate_tx_double_spend() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let mut chain = Chain::new(&key_1.public_key());
        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];

        let invalid_tx = Transaction::new(TransactionData::new(
            vec![
                Input {
                    hash: last_coinbase.hash.clone(),
                    index: 0,
                    signature: key_1.sign(last_coinbase.hash.digest()),
                },
                Input {
                    hash: last_coinbase.hash.clone(),
                    index: 0,
                    signature: key_1.sign(last_coinbase.hash.digest()),
                },
            ],
            vec![Output {
                value: 20000,
                pubkey: key_2.public_key(),
            }],
        ));

        let result = chain.add_block(Block::new(BlockData::new(
            last_block.hash.clone(),
            0,
            vec![invalid_tx],
        )));

        println!("{:#?}", chain);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ChainOpError::InvalidBlock);
    }

    #[test]
    fn validate_block_double_spend() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let chain = Chain::new(&key_1.public_key());
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

        let block = Block {
            hash: Hash::new(b"test"),
            data: BlockData::new(last_block.hash.clone(), 0, vec![tx.clone(), tx]),
        };
        assert!(!chain.validate_double_spend(&block.data.transactions));
    }

    #[test]
    fn validate_new_block() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let chain = Chain::new(&key_1.public_key());
        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];

        let valid_tx = Transaction::new(TransactionData::new(
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
        let valid_coinbase_tx = Transaction::new(TransactionData::new_with_timestamp(
            vec![],
            vec![Output {
                value: chain.rules.base_coins,
                pubkey: key_1.public_key(),
            }],
            0,
        ));

        let block = Block {
            hash: Hash::new(b"test"),
            data: BlockData::new(last_block.hash.clone(), 0, vec![valid_tx.clone()]),
        };

        assert!(!chain.validate_new_block(&block));

        let block = Block {
            hash: Hash::new(b"test"),
            data: BlockData::new(last_block.hash.clone(), 0, vec![valid_tx.clone()]),
        };
        assert!(!chain.validate_new_block(&block));

        let block = Block::new(BlockData::new(
            Hash::new(b"test"),
            0,
            vec![valid_tx.clone()],
        ));
        assert!(!chain.validate_new_block(&block));

        let block = Block::new(BlockData::new(
            last_block.hash.clone(),
            0,
            vec![Transaction::new(TransactionData::new(
                vec![],
                vec![Output {
                    value: 5000,
                    pubkey: key_2.public_key(),
                }],
            ))],
        ));
        assert!(!chain.validate_new_block(&block));

        let block = Block::new(BlockData::new(
            last_block.hash.clone(),
            0,
            vec![valid_tx.clone()],
        ));
        assert!(chain.validate_new_block(&block));

        let block = Block::new(BlockData::new(
            last_block.hash.clone(),
            0,
            vec![valid_tx.clone(), valid_coinbase_tx.clone()],
        ));
        assert!(chain.validate_new_block(&block));

        let block = Block::new(BlockData::new(
            last_block.hash.clone(),
            0,
            vec![valid_tx.clone(), valid_tx.clone()],
        ));
        assert!(!chain.validate_new_block(&block));
    }

    #[test]
    fn validate_chain() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let mut chain = Chain::new(&key_1.public_key());
        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];

        let valid_tx = Transaction::new(TransactionData::new(
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
        let valid_coinbase_tx = Transaction::new(TransactionData::new_with_timestamp(
            vec![],
            vec![Output {
                value: chain.rules.base_coins,
                pubkey: key_1.public_key(),
            }],
            0,
        ));

        let result = chain.chain.append(Block::new(BlockData::new(
            last_block.hash.clone(),
            0,
            vec![valid_tx, valid_coinbase_tx.clone()],
        )));

        assert!(result.is_ok());
        assert!(chain.validate_chain());
    }

    #[test]
    fn add_block() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let mut chain = Chain::new_with_consensus(
            &key_1.public_key(),
            ConsensusRules::new(Target::from_leading_zeros(254), 10000, Halving::None),
        );
        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];
        let last_block_hash = last_block.hash.clone();

        let valid_tx = Transaction::new(TransactionData::new(
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
        let valid_coinbase_tx = Transaction::new(TransactionData::new_with_timestamp(
            vec![],
            vec![Output {
                value: chain.rules.base_coins + 5000,
                pubkey: key_1.public_key(),
            }],
            0,
        ));

        let result = chain.add_block(Block::new(BlockData::new(
            Hash::new(&U256::from(2_u32).to_be_bytes()),
            0,
            vec![valid_tx.clone(), valid_coinbase_tx.clone()],
        )));

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ChainOpError::TargetNotSatisfied);

        chain.rules = ConsensusRules::default();

        let result = chain.add_block(Block::new(BlockData::new(
            last_block_hash,
            0,
            vec![valid_tx, valid_coinbase_tx],
        )));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        let utxos_1 = chain.find_utxos_for_key(&key_1.public_key());
        assert_eq!(
            utxos_1.iter().fold(0, |acc, u| acc + u.value),
            (chain.rules.base_coins * 2) - 5000
        );

        let utxos_2 = chain.find_utxos_for_key(&key_2.public_key());
        assert_eq!(utxos_2.iter().fold(0, |acc, u| acc + u.value), 5000);
    }

    #[test]
    fn halving_inf() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let mut chain = Chain::new_with_consensus(
            &key_1.public_key(),
            ConsensusRules::new(Target::MAX, 10000, Halving::Inf),
        );
        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];
        let last_block_hash = last_block.hash.clone();

        let valid_tx = Transaction::new(TransactionData::new(
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
        let invalid_coinbase_tx = Transaction::new(TransactionData::new_with_timestamp(
            vec![],
            vec![Output {
                value: 5001,
                pubkey: key_1.public_key(),
            }],
            0,
        ));

        let result = chain.add_block(Block::new(BlockData::new(
            last_block_hash.clone(),
            0,
            vec![valid_tx.clone(), invalid_coinbase_tx],
        )));

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ChainOpError::InvalidBlock);

        let valid_coinbase_tx = Transaction::new(TransactionData::new_with_timestamp(
            vec![],
            vec![Output {
                value: 5000,
                pubkey: key_1.public_key(),
            }],
            0,
        ));

        let result = chain.add_block(Block::new(BlockData::new(
            last_block_hash,
            0,
            vec![valid_tx, valid_coinbase_tx],
        )));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        let utxos_1 = chain.find_utxos_for_key(&key_1.public_key());
        assert_eq!(
            utxos_1.iter().fold(0, |acc, u| acc + u.value),
            chain.rules.base_coins - 5000
        );

        let utxos_2 = chain.find_utxos_for_key(&key_2.public_key());
        assert_eq!(utxos_2.iter().fold(0, |acc, u| acc + u.value), 5000);
    }

    #[test]
    fn halving_height() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let mut chain = Chain::new_with_consensus(
            &key_1.public_key(),
            ConsensusRules::new(Target::MAX, 10000, Halving::Height(1)),
        );
        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];
        let last_block_hash = last_block.hash.clone();

        let valid_tx = Transaction::new(TransactionData::new(
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
        let invalid_coinbase_tx = Transaction::new(TransactionData::new_with_timestamp(
            vec![],
            vec![Output {
                value: 10001,
                pubkey: key_1.public_key(),
            }],
            0,
        ));

        let result = chain.add_block(Block::new(BlockData::new(
            last_block_hash.clone(),
            0,
            vec![valid_tx.clone(), invalid_coinbase_tx.clone()],
        )));

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ChainOpError::InvalidBlock);

        let valid_coinbase_tx = Transaction::new(TransactionData::new_with_timestamp(
            vec![],
            vec![Output {
                value: 10000,
                pubkey: key_1.public_key(),
            }],
            0,
        ));

        let result = chain.add_block(Block::new(BlockData::new(
            last_block_hash,
            0,
            vec![valid_tx, valid_coinbase_tx],
        )));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        let utxos_1 = chain.find_utxos_for_key(&key_1.public_key());
        assert_eq!(
            utxos_1.iter().fold(0, |acc, u| acc + u.value),
            chain.rules.base_coins + (chain.rules.base_coins / 2) - 5000
        );

        let utxos_2 = chain.find_utxos_for_key(&key_2.public_key());
        assert_eq!(utxos_2.iter().fold(0, |acc, u| acc + u.value), 5000);
    }

    #[test]
    fn chain_test() {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let keys = vec![
            KeyPair::new(),
            KeyPair::new(),
            KeyPair::new(),
            KeyPair::new(),
            KeyPair::new(),
        ];

        let mut chain = Chain::new(&keys[0].public_key());

        let iterations = 3;
        let mut rng = thread_rng();
        for _ in 0..iterations {
            let accounts: Vec<(KeyPair, Vec<Utxo>, Value)> = keys
                .iter()
                .map(|key: &KeyPair| {
                    let utxos = chain.find_utxos_for_key(&key.public_key());
                    let value = utxos.iter().fold(0, |acc, utxo| acc + utxo.value);
                    (key.clone(), utxos, value)
                })
                .collect();

            let mut transactions = Vec::new();
            for account in &accounts {
                println!(
                    "Account: {:?}\n Value: {}",
                    account.0.public_key(),
                    account.2
                );
                let tx_count = 4;
                let tx_value = account.2 / tx_count;
                let tx_rem = account.2 % tx_count;

                let tx = new_tx(
                    &account.0,
                    &account.1,
                    (0..tx_count)
                        .map(|id| Output {
                            value: if id != tx_count - 1 {
                                tx_value
                            } else {
                                tx_value + tx_rem
                            },
                            pubkey: accounts.choose(&mut rng).unwrap().0.public_key(),
                        })
                        .collect(),
                );

                if tx.is_ok() {
                    transactions.push(tx.unwrap());
                } else {
                    println!("Transaction not added");
                }
            }

            let block = new_block(&chain, 0, transactions);
            println!("{:#?}", block);
            match chain.add_block(block) {
                Ok(height) => println!("Added block: {}", height),
                Err(_) => println!("Block not added"),
            }

            assert_eq!(
                accounts.iter().fold(0, |tot, account| tot + account.2),
                chain.rules.base_coins
            );
        }
        assert_eq!(chain.height(), iterations + 1);
    }
}
