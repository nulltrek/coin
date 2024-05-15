use crate::consensus::ConsensusRules;
use crate::traits::io::{ByteIO, FileIO};
use crate::types::block::Block;
use crate::types::blockchain::{Blockchain, BlockchainError};
use crate::types::keys::PublicKey;
use crate::types::transaction::{Output, Transaction};
use crate::utils::*;
use crate::utxo::Utxo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Debug)]
pub enum ChainOpError {
    TargetNotSatisfied,
    InvalidBlock,
    InvalidPrevHash,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chain {
    pub rules: ConsensusRules,
    pub chain: Blockchain,
}

impl Chain {
    pub fn new(pubkey: &PublicKey) -> Chain {
        let rules = ConsensusRules::default();
        let genesis = new_genesis_block(pubkey, rules.coins_per_block);
        Chain {
            rules,
            chain: Blockchain::new(genesis),
        }
    }

    pub fn new_with_consensus(pubkey: &PublicKey, rules: ConsensusRules) -> Chain {
        let genesis = new_genesis_block(pubkey, rules.coins_per_block);
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
            && match self.chain.get_tx_value(&genesis.data.transactions[0]) {
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
            && match self.chain.get_tx_value(tx) {
                Some(value) => value.output > 0 && value.input >= value.output,
                None => false,
            };
    }

    /*
     * A coinbase transaction is valid if:
     * - Its hash is valid
     * - There are no inputs
     * - There is at least 1 output
     * - The total output value is less than or equal to the coins per block consensus rule + fees on the block
     */
    fn validate_coinbase_tx(&self, block: &Block, tx: &Transaction) -> bool {
        let block_value = self.chain.get_block_value(block);
        if block_value.is_none() {
            return false;
        }
        return tx.is_hash_valid()
            && tx.data.inputs.len() == 0
            && tx.data.outputs.len() > 0
            && match self.chain.get_tx_value(tx) {
                Some(value) => {
                    value.input == 0
                        && value.output <= (self.rules.coins_per_block + block_value.unwrap().fees)
                }
                None => false,
            };
    }

    /*
     * A block is valid if:
     * - Its hash is valid
     * - The block points to the previous block
     * - There is at least one transaction
     * - If there is only one transaction, it's a regular transaction
     * - The top hash is valid
     * - All the transactions except the last one are valid regular transactions
     * - The last transaction is a valid coinbase transaction or a valid regular transaction
     */
    fn validate_block_with_previous(&self, block: &Block, previous: &Block) -> bool {
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
                .fold(true, |acc, tx| acc && self.validate_tx(tx))
            && (self.validate_coinbase_tx(block, block.data.transactions.last().unwrap())
                || self.validate_tx(block.data.transactions.last().unwrap()));
    }

    pub fn validate_block(&self, block: &Block) -> bool {
        self.validate_block_with_previous(block, self.chain.get_last_block())
    }

    /*
     * A chain is valid if:
     * - The genesis block is valid
     * - All the remaining blocks are valid
     */
    pub fn validate_chain(&self) -> bool {
        return self.validate_genesis()
            && self.chain.list[1..]
                .iter()
                .enumerate()
                .fold(true, |acc, (i, block)| {
                    acc && self.validate_block_with_previous(block, &self.chain.list[i])
                });
    }

    /*
     * A block can be added to the blockchain if:
     * - Its hash satisfies the consensus target
     * - It's a valid block
     */
    pub fn add_block(&mut self, block: Block) -> Result<usize, ChainOpError> {
        if !self.rules.validate_target(&block.hash) {
            return Err(ChainOpError::TargetNotSatisfied);
        }
        if !self.validate_block(&block) {
            return Err(ChainOpError::InvalidBlock);
        }

        match self.chain.append(block) {
            Err(BlockchainError::InvalidPrevHash) => Err(ChainOpError::InvalidPrevHash),
            Ok(value) => Ok(value),
        }
    }
}

impl ByteIO for Chain {}
impl FileIO for Chain {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::block::{Block, BlockData};
    use crate::types::hash::Hash;
    use crate::types::keys::KeyPair;
    use crate::types::transaction::{Input, Output, TransactionData};
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
        assert_eq!(utxos[0].value, chain.rules.coins_per_block);

        let utxos = chain.find_utxos_for_key(&key.public_key());
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].value, chain.rules.coins_per_block);

        let utxos = chain.find_utxos_for_key(&KeyPair::new().public_key());
        assert_eq!(utxos.len(), 0);
    }

    #[test]
    fn validate_tx() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();
        let chain = Chain::new(&key_1.public_key());
        let coinbase = &chain.chain.list[0].data.transactions[0];

        let tx = Transaction::new(TransactionData {
            inputs: vec![Input {
                hash: coinbase.hash.clone(),
                index: 0,
                signature: key_1.sign(coinbase.hash.digest()),
            }],
            outputs: vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        });
        assert!(chain.validate_tx(&tx));

        let tx = Transaction::new(TransactionData {
            inputs: vec![],
            outputs: vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        });
        assert!(!chain.validate_tx(&tx));

        let tx = Transaction::new(TransactionData {
            inputs: vec![Input {
                hash: coinbase.hash.clone(),
                index: 0,
                signature: key_1.sign(coinbase.hash.digest()),
            }],
            outputs: vec![],
        });
        assert!(!chain.validate_tx(&tx));

        let tx = Transaction::new(TransactionData {
            inputs: vec![Input {
                hash: coinbase.hash.clone(),
                index: 0,
                signature: key_2.sign(coinbase.hash.digest()),
            }],
            outputs: vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        });
        assert!(!chain.validate_tx(&tx));

        let tx = Transaction::new(TransactionData {
            inputs: vec![Input {
                hash: coinbase.hash.clone(),
                index: 0,
                signature: key_1.sign(coinbase.hash.digest()),
            }],
            outputs: vec![Output {
                value: 0,
                pubkey: key_2.public_key(),
            }],
        });
        assert!(!chain.validate_tx(&tx));

        let tx = Transaction::new(TransactionData {
            inputs: vec![Input {
                hash: coinbase.hash.clone(),
                index: 0,
                signature: key_1.sign(coinbase.hash.digest()),
            }],
            outputs: vec![Output {
                value: chain.rules.coins_per_block + 1,
                pubkey: key_2.public_key(),
            }],
        });
        assert!(!chain.validate_tx(&tx));
    }

    #[test]
    fn validate_coinbase_tx() {
        let key = KeyPair::new();
        let chain = Chain::new(&key.public_key());

        let genesis = &chain.chain.list[0];
        let coinbase = &genesis.data.transactions[0];
        assert!(chain.validate_coinbase_tx(genesis, &coinbase));

        let tx = Transaction {
            hash: Hash::new(b"test"),
            data: coinbase.data.clone(),
        };
        assert!(!chain.validate_coinbase_tx(genesis, &tx));

        let tx = Transaction::new(TransactionData {
            inputs: vec![],
            outputs: vec![],
        });
        assert!(!chain.validate_coinbase_tx(genesis, &tx));

        let tx = Transaction::new(TransactionData {
            inputs: vec![],
            outputs: vec![Output {
                value: chain.rules.coins_per_block,
                pubkey: key.public_key(),
            }],
        });
        assert!(chain.validate_coinbase_tx(genesis, &tx));

        let tx = Transaction::new(TransactionData {
            inputs: vec![],
            outputs: vec![Output {
                value: chain.rules.coins_per_block + 5000,
                pubkey: key.public_key(),
            }],
        });
        let block = Block::new(BlockData::new(
            genesis.hash.clone(),
            0,
            vec![Transaction::new(TransactionData {
                inputs: vec![Input {
                    hash: coinbase.hash.clone(),
                    index: 0,
                    signature: key.sign(coinbase.hash.digest()),
                }],
                outputs: vec![Output {
                    value: 5000,
                    pubkey: key.public_key(),
                }],
            })],
        ));

        assert!(chain.validate_coinbase_tx(&block, &tx));

        let tx = Transaction::new(TransactionData {
            inputs: vec![],
            outputs: vec![Output {
                value: chain.rules.coins_per_block,
                pubkey: key.public_key(),
            }],
        });

        assert!(chain.validate_coinbase_tx(&block, &tx));

        let tx = Transaction::new(TransactionData {
            inputs: vec![],
            outputs: vec![Output {
                value: chain.rules.coins_per_block + 5001,
                pubkey: key.public_key(),
            }],
        });

        assert!(!chain.validate_coinbase_tx(&block, &tx));
    }

    #[test]
    fn validate_block() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let chain = Chain::new(&key_1.public_key());
        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];

        let valid_tx = Transaction::new(TransactionData {
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
        let valid_coinbase_tx = Transaction::new(TransactionData {
            inputs: vec![],
            outputs: vec![Output {
                value: chain.rules.coins_per_block,
                pubkey: key_1.public_key(),
            }],
        });

        let block = Block {
            hash: Hash::new(b"test"),
            data: BlockData::new(last_block.hash.clone(), 0, vec![valid_tx.clone()]),
        };
        assert!(!chain.validate_block(&block));

        let block = Block::new(BlockData::new(
            Hash::new(b"test"),
            0,
            vec![valid_tx.clone()],
        ));
        assert!(!chain.validate_block(&block));

        let block = Block::new(BlockData::new(
            last_block.hash.clone(),
            0,
            vec![Transaction::new(TransactionData {
                inputs: vec![],
                outputs: vec![Output {
                    value: 5000,
                    pubkey: key_2.public_key(),
                }],
            })],
        ));
        assert!(!chain.validate_block(&block));

        let block = Block::new(BlockData::new(
            last_block.hash.clone(),
            0,
            vec![valid_tx.clone()],
        ));
        assert!(chain.validate_block(&block));

        let block = Block::new(BlockData::new(
            last_block.hash.clone(),
            0,
            vec![valid_tx.clone(), valid_coinbase_tx.clone()],
        ));
        assert!(chain.validate_block(&block));

        let block = Block::new(BlockData::new(
            last_block.hash.clone(),
            0,
            vec![valid_tx.clone(), valid_tx.clone()],
        ));
        assert!(chain.validate_block(&block));
    }

    #[test]
    fn validate_chain() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let mut chain = Chain::new(&key_1.public_key());
        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];

        let valid_tx = Transaction::new(TransactionData {
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
        let valid_coinbase_tx = Transaction::new(TransactionData {
            inputs: vec![],
            outputs: vec![Output {
                value: chain.rules.coins_per_block,
                pubkey: key_1.public_key(),
            }],
        });

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

        let mut chain =
            Chain::new_with_consensus(&key_1.public_key(), ConsensusRules::new(U256::from(1_u32)));
        let last_block = chain.chain.get_last_block();
        let last_coinbase = &last_block.data.transactions[0];
        let last_block_hash = last_block.hash.clone();

        let valid_tx = Transaction::new(TransactionData {
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
        let valid_coinbase_tx = Transaction::new(TransactionData {
            inputs: vec![],
            outputs: vec![Output {
                value: chain.rules.coins_per_block,
                pubkey: key_1.public_key(),
            }],
        });

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
    }
}
