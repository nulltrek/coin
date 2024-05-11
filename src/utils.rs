use crate::consensus::ConsensusRules;
use crate::types::block::{Block, BlockData};
use crate::types::blockchain::Blockchain;
use crate::types::hash::Hash;
use crate::types::keys::KeyPair;
use crate::types::keys::{PublicKey, Verifier};
use crate::types::transaction::{Output, Transaction, TransactionData};
use crate::utxo::{IntoInputs, Utxo, UtxoError};
use std::ops::Add;

#[derive(Debug, PartialEq, Eq)]
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

pub fn get_tx_output_value(outputs: &Vec<Output>) -> u64 {
    outputs.iter().fold(0, |acc, o| acc + o.value)
}

pub fn get_tx_value(chain: &Blockchain, tx: &Transaction) -> Option<TotalValue> {
    let input: u64 = match get_tx_input_value(chain, tx) {
        Some(value) => value,
        None => return None,
    };
    let output = get_tx_output_value(&tx.data.outputs);
    if input > 0 && output > input {
        return None;
    }
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

pub fn new_tx(
    key: &KeyPair,
    utxos: &[Utxo],
    mut outputs: Vec<Output>,
) -> Result<Transaction, UtxoError> {
    let value = get_tx_output_value(&outputs);
    let selection = Utxo::collect(utxos, value)?;
    let inputs = selection.list.into_inputs(key);
    outputs.push(Output {
        value: selection.change,
        pubkey: key.public_key(),
    });
    Ok(Transaction::new(TransactionData { inputs, outputs }))
}

pub fn verify_tx_signatures(chain: &Blockchain, tx: &Transaction) -> bool {
    for input in &tx.data.inputs {
        let idx = input.index as usize;
        let (_, input_tx) = match chain.query_tx(&input.hash) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Chain;
    use crate::types::keys::KeyPair;
    use crate::types::transaction::Input;

    #[test]
    fn tx_value() {
        let key = KeyPair::new();
        let cr = ConsensusRules::default();
        let tx = new_coinbase_tx(&cr, &key.public_key());

        assert_eq!(get_tx_output_value(&tx.data.outputs), cr.coins_per_block);

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
        assert_eq!(get_tx_output_value(&tx.data.outputs), 77);
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

    #[test]
    fn tx_get_value() {
        let key = KeyPair::new();
        let chain = Chain::new(&key.public_key());
        let coinbase = &chain.chain.list[0].data.transactions[0];

        let make_tx = |hash: &Hash, value: u64| {
            Transaction::new(TransactionData {
                inputs: vec![Input {
                    hash: hash.clone(),
                    index: 0,
                    signature: key.sign(hash.digest()),
                }],
                outputs: vec![Output {
                    value: value,
                    pubkey: key.public_key(),
                }],
            })
        };

        let tx = make_tx(&coinbase.hash, 5000);
        let value = get_tx_value(&chain.chain, &tx);
        assert!(value.is_some());
        assert_eq!(
            get_tx_value(&chain.chain, &tx).unwrap(),
            TotalValue {
                input: chain.rules.coins_per_block,
                output: 5000,
                fees: chain.rules.coins_per_block - 5000,
            }
        );

        let tx = make_tx(&coinbase.hash, chain.rules.coins_per_block);
        let value = get_tx_value(&chain.chain, &tx);
        assert!(value.is_some());
        assert_eq!(
            get_tx_value(&chain.chain, &tx).unwrap(),
            TotalValue {
                input: chain.rules.coins_per_block,
                output: chain.rules.coins_per_block,
                fees: 0,
            }
        );

        let tx = make_tx(&coinbase.hash, chain.rules.coins_per_block + 1);
        let value = get_tx_value(&chain.chain, &tx);
        assert!(value.is_none());
    }

    #[test]
    fn tx_creation() {
        let key_1 = KeyPair::new();
        let key_2 = KeyPair::new();

        let chain = Chain::new(&key_1.public_key());

        let utxos = chain.find_utxos_for_key(&key_1.public_key());

        let tx = new_tx(
            &key_1,
            &utxos,
            vec![Output {
                value: 20000,
                pubkey: key_2.public_key().clone(),
            }],
        );

        assert!(tx.is_err());

        let tx = new_tx(
            &key_1,
            &utxos,
            vec![Output {
                value: 7000,
                pubkey: key_2.public_key().clone(),
            }],
        );

        assert!(tx.is_ok());

        let tx = tx.unwrap();
        assert_eq!(tx.data.outputs.len(), 2);
        assert_eq!(tx.data.outputs[0].value, 7000);
        assert_eq!(tx.data.outputs[0].pubkey, key_2.public_key());
        assert_eq!(tx.data.outputs[1].value, 3000);
        assert_eq!(tx.data.outputs[1].pubkey, key_1.public_key());
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

        assert!(verify_tx_signatures(&chain.chain, &tx.unwrap()));

        let tx = new_tx(
            &key_2,
            &utxos,
            vec![Output {
                value: 5000,
                pubkey: key_2.public_key(),
            }],
        )
        .unwrap();

        assert!(!verify_tx_signatures(&chain.chain, &tx));
    }
}
