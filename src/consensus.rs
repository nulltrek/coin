use crate::core::hash::Hash;
use crate::core::transaction::Value;
use crate::traits::io::{ByteIO, FileIO};
use core::fmt::Display;
use ethnum::U256;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Write;
use std::fmt::{self, Binary, Formatter};

#[derive(PartialEq, PartialOrd, Clone, Debug)]
pub struct Target {
    pub value: U256,
}

impl Target {
    const MAX: Target = Target { value: U256::MAX };

    pub fn from_hash(hash: &Hash) -> Target {
        Target {
            value: U256::from_be_bytes(hash.digest().clone()),
        }
    }

    pub fn from_leading_zeros(count: u8) -> Target {
        Target {
            value: U256::MAX >> count,
        }
    }

    pub fn leading_zeros(&self) -> u32 {
        self.value.leading_zeros()
    }
}

impl Binary for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        let val = self.value;
        fmt::Binary::fmt(&val, f)
    }
}

impl Serialize for Target {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut f = String::new();
        write!(&mut f, "{:#x}", self.value).expect("unexpected formatting failure");
        serializer.serialize_str(f.as_str())
    }
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TargetVisitor(U256::from_str_hex))
    }
}

struct TargetVisitor<F>(F);

impl<'de, E, F> Visitor<'de> for TargetVisitor<F>
where
    E: Display,
    F: FnOnce(&str) -> Result<U256, E>,
{
    type Value = Target;

    fn expecting(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("a formatted 256-bit integer")
    }

    fn visit_str<E_>(self, v: &str) -> Result<Self::Value, E_>
    where
        E_: de::Error,
    {
        match self.0(v) {
            Ok(value) => Ok(Target { value }),
            Err(e) => Err(de::Error::custom(e)),
        }
    }

    fn visit_bytes<E_>(self, v: &[u8]) -> Result<Self::Value, E_>
    where
        E_: de::Error,
    {
        let string = std::str::from_utf8(v)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Bytes(v), &self))?;
        self.visit_str(string)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConsensusRules {
    pub target: Target,
    pub coins_per_block: Value,
}

impl Default for ConsensusRules {
    fn default() -> ConsensusRules {
        ConsensusRules {
            target: Target::MAX,
            coins_per_block: 10000,
        }
    }
}

impl ConsensusRules {
    pub fn new(target: Target) -> ConsensusRules {
        ConsensusRules {
            target: target,
            ..ConsensusRules::default()
        }
    }

    pub fn new_with_leading(leading_zeros: u8) -> ConsensusRules {
        ConsensusRules {
            target: Target::from_leading_zeros(leading_zeros),
            ..ConsensusRules::default()
        }
    }

    pub fn validate_target(&self, hash: &Hash) -> bool {
        Target::from_hash(hash) <= self.target
    }
}

impl ByteIO for ConsensusRules {}
impl FileIO for ConsensusRules {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::io::ByteIO;

    #[test]
    fn target_cmp() {
        let hash = Hash::from_bytes(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ])
        .unwrap();

        let cr = ConsensusRules::default();
        assert!(cr.validate_target(&hash));

        let hash = Hash::from_bytes(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 3,
        ])
        .unwrap();

        let cr = ConsensusRules::new(Target::from_leading_zeros(255));
        assert!(!cr.validate_target(&hash));

        let hash = Hash::from_bytes(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
            0, 0, 0,
        ])
        .unwrap();

        let cr = ConsensusRules::new(Target::from_leading_zeros(198));
        assert!(cr.validate_target(&hash));
        let cr = ConsensusRules::new(Target::from_leading_zeros(200));
        assert!(!cr.validate_target(&hash));
    }

    #[test]
    fn serde() {
        let target = Target::MAX;

        let result = serde_json::to_string(&target).unwrap();
        let deserialized = serde_json::from_str(result.as_str()).unwrap();
        assert_eq!(target, deserialized);

        let result = bincode::serialize(&target).unwrap();
        let deserialized = bincode::deserialize(result.as_slice()).unwrap();
        assert_eq!(target, deserialized);

        let target = Target::from_leading_zeros(128);

        let result = serde_json::to_string(&target).unwrap();
        let deserialized = serde_json::from_str(result.as_str()).unwrap();
        assert_eq!(target, deserialized);

        let result = bincode::serialize(&target).unwrap();
        let deserialized = bincode::deserialize(result.as_slice()).unwrap();
        assert_eq!(target, deserialized);
    }
}
