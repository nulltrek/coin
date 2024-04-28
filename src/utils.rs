use crate::consensus::ConsensusRules;
use crate::types::block::{Block, BlockData};
use crate::types::blockchain::Blockchain;
use crate::types::hash::Hash;
use crate::types::keys::PublicKey;
use crate::types::transaction::{Output, Transaction, TransactionData};
use std::ops::Add;

pub struct TotalValue {
    pub input: u64,
    pub output: u64,
    pub fees: u64,
}

impl Default for TotalValue {
    fn default() -> TotalValue {
        TotalValue {
            input: 0,
            output: 0,
            fees: 0,
        }
    }
}

impl TotalValue {
    pub fn new(input: u64, output: u64, fees: u64) -> TotalValue {
        TotalValue {
            input,
            output,
            fees,
        }
    }
}

impl Add for TotalValue {
    type Output = TotalValue;

    fn add(self, rhs: Self) -> Self::Output {
        TotalValue {
            input: self.input + rhs.input,
            output: self.output + rhs.output,
            fees: self.fees + rhs.fees,
        }
    }
}

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

pub fn get_tx_input_value(chain: &Blockchain, tx: &Transaction) -> Option<u64> {
    let mut value: u64 = 0;
    for input in &tx.data.inputs {
        let result = chain.query_tx(&input.hash);
        if result.is_none() {
            return None;
        }
        let (_, tx) = result.unwrap();
        if tx.data.outputs.len() <= input.index as usize {
            return None;
        }
        value += tx.data.outputs[input.index as usize].value;
    }
    Some(value)
}

pub fn get_tx_output_value(tx: &Transaction) -> u64 {
    tx.data.outputs.iter().fold(0, |acc, o| acc + o.value)
}

pub fn get_tx_value(chain: &Blockchain, tx: &Transaction) -> Option<TotalValue> {
    let input: u64 = match get_tx_input_value(chain, tx) {
        Some(value) => value,
        None => return None,
    };
    let output = get_tx_output_value(tx);
    let fees = if input == 0 { 0 } else { input - output };
    Some(TotalValue::new(input, output, fees))
}

pub fn get_block_value(chain: &Blockchain, block: &Block) -> Option<TotalValue> {
    let mut acc = TotalValue::default();
    for tx in block.data.transactions.iter() {
        let result = get_tx_value(chain, tx);
        if result.is_none() {
            return None;
        }
        acc = acc + result.unwrap();
    }
    Some(acc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::keys::KeyPair;
    use crate::types::transaction::Input;

    #[test]
    fn tx_value() {
        let key = KeyPair::new();
        let cr = ConsensusRules::default();
        let tx = new_coinbase_tx(&cr, &key.public_key());

        assert_eq!(get_tx_output_value(&tx), cr.coins_per_block);

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
        assert_eq!(get_tx_output_value(&tx), 77);
    }

    #[test]
    fn block_value() {
        let key = KeyPair::new();
        let cr = ConsensusRules::default();

        let genesis = new_genesis_block(&cr, &key.public_key());
        let genesis_hash = genesis.hash.clone();
        let coinbase_hash = genesis.data.transactions[0].hash.clone();

        let chain = Blockchain::new(genesis);

        let next = Block::new(BlockData::new(
            genesis_hash,
            0,
            vec![
                Transaction::new(TransactionData {
                    inputs: vec![Input {
                        hash: coinbase_hash.clone(),
                        index: 0,
                        signature: key.sign(coinbase_hash.digest()),
                    }],
                    outputs: vec![
                        Output {
                            value: 5000,
                            pubkey: key.public_key(),
                        },
                        Output {
                            value: 4000,
                            pubkey: key.public_key(),
                        },
                    ],
                }),
                new_coinbase_tx(&cr, &key.public_key()),
            ],
        ));

        let value = get_block_value(&chain, &next).unwrap();

        assert_eq!(value.input, cr.coins_per_block);
        assert_eq!(value.output, 19000);
        assert_eq!(value.fees, 1000);
    }
}
