use crate::chain::Chain;
use crate::core::block::{Block, BlockData, Nonce};
use crate::core::blockchain::Blockchain;
use crate::core::hash::Hash;
use crate::core::keys::KeyPair;
use crate::core::keys::{PublicKey, Verifier};
use crate::core::transaction::{Output, Transaction, TransactionData, Value};
use crate::utxo::{IntoInputs, Utxo, UtxoError};

pub fn new_coinbase_tx(pubkey: &PublicKey, value: Value) -> Transaction {
    Transaction::new(TransactionData {
        inputs: Vec::new(),
        outputs: vec![Output {
            value,
            pubkey: pubkey.clone(),
        }],
    })
}

pub fn new_genesis_block(pubkey: &PublicKey, coinbase_value: Value) -> Block {
    Block::new(BlockData::new(
        Hash::default(),
        0,
        vec![new_coinbase_tx(pubkey, coinbase_value)],
    ))
}

pub fn new_tx(
    key: &KeyPair,
    utxos: &[Utxo],
    mut outputs: Vec<Output>,
) -> Result<Transaction, UtxoError> {
    let value = Blockchain::get_tx_output_value(&outputs);
    let selection = Utxo::collect(utxos, value)?;
    let inputs = selection.list.into_inputs(key);
    if selection.change != 0 {
        outputs.push(Output {
            value: selection.change,
            pubkey: key.public_key(),
        });
    }
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

pub fn new_block(chain: &Chain, nonce: Nonce, transactions: Vec<Transaction>) -> Block {
    Block::new(BlockData::new(
        chain.get_last_block().hash.clone(),
        nonce,
        transactions,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::keys::KeyPair;

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

        let tx = new_tx(
            &key_1,
            &utxos,
            vec![Output {
                value: 10000,
                pubkey: key_2.public_key().clone(),
            }],
        );

        assert!(tx.is_ok());

        let tx = tx.unwrap();
        assert_eq!(tx.data.outputs.len(), 1);
        assert_eq!(tx.data.outputs[0].value, 10000);
        assert_eq!(tx.data.outputs[0].pubkey, key_2.public_key());
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
