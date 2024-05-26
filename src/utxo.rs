use crate::core::hash::Hash;
use crate::core::keys::KeyPair;
use crate::core::transaction::Input;
use crate::core::transaction::Value;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(PartialEq, Debug)]
pub enum UtxoError {
    InvalidValue,
    NotEnoughValue,
    InvalidTransaction,
}

impl fmt::Display for UtxoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Utxo error: {:?}", self)
    }
}

pub struct UtxoSelection<'a> {
    pub list: &'a [Utxo],
    pub change: Value,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Debug)]
pub struct Utxo {
    pub hash: Hash,
    pub output: u32,
    pub value: Value,
}

impl Utxo {
    pub fn new(hash: Hash, output: u32, value: Value) -> Utxo {
        Utxo {
            hash,
            output,
            value,
        }
    }

    pub fn collect(utxos: &[Utxo], value: Value) -> Result<UtxoSelection, UtxoError> {
        if value == 0 {
            return Err(UtxoError::InvalidValue);
        }

        let mut acc: Value = 0;
        let mut last: usize = 0;
        for (idx, utxo) in utxos.iter().enumerate() {
            last = idx;
            acc += utxo.value;
            if acc >= value {
                break;
            }
        }

        if acc < value {
            return Err(UtxoError::NotEnoughValue);
        }

        Ok(UtxoSelection {
            list: &utxos[..last + 1],
            change: acc - value,
        })
    }

    // pub fn sign(&self, key: &KeyPair) -> Signature {
    //     key.sign(self.hash.digest())
    // }
}

pub trait IntoInputs {
    fn into_inputs(&self, key: &KeyPair) -> Vec<Input>;
}

impl IntoInputs for &[Utxo] {
    fn into_inputs(&self, key: &KeyPair) -> Vec<Input> {
        self.into_iter()
            .map(|utxo| Input {
                hash: utxo.hash.clone(),
                index: utxo.output,
                signature: key.sign(utxo.hash.digest()),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Chain;

    #[test]
    fn collect_value() {
        let mut utxos = vec![
            Utxo::new(Hash::new(b"test"), 0, 100),
            Utxo::new(Hash::new(b"test"), 0, 50),
            Utxo::new(Hash::new(b"test"), 0, 1),
            Utxo::new(Hash::new(b"test"), 0, 2320),
        ];

        match Utxo::collect(&utxos, 0) {
            Ok(_) => assert!(false),
            Err(err) => assert_eq!(err, UtxoError::InvalidValue),
        }

        match Utxo::collect(&utxos, 10000) {
            Ok(_) => assert!(false),
            Err(err) => assert_eq!(err, UtxoError::NotEnoughValue),
        }

        match Utxo::collect(&utxos, 100) {
            Ok(UtxoSelection { list, change }) => {
                assert_eq!(list.len(), 1);
                assert_eq!(change, 0);
            }
            Err(_) => assert!(false),
        }

        match Utxo::collect(&utxos, 101) {
            Ok(UtxoSelection { list, change }) => {
                assert_eq!(list.len(), 2);
                assert_eq!(change, 49);
            }
            Err(_) => assert!(false),
        }

        utxos.sort_by(|a, b| a.value.cmp(&b.value));

        match Utxo::collect(&utxos, 51) {
            Ok(UtxoSelection { list, change }) => {
                assert_eq!(list.len(), 2);
                assert_eq!(change, 0);
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn into_inputs() {
        let key = KeyPair::new();
        let chain = Chain::new(&key.public_key());

        let genesis = chain.get_block(0).unwrap();

        let signature = key.sign(genesis.data.transactions[0].hash.digest());

        let utxos = chain.find_utxos_for_key(&key.public_key());
        let inputs = utxos.as_slice().into_inputs(&key);

        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].hash, genesis.data.transactions[0].hash);
        assert_eq!(inputs[0].index, 0);
        assert_eq!(inputs[0].signature, signature);
    }
}
