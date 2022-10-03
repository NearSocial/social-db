use crate::*;

pub(crate) mod unordered_map_expensive {
    use super::*;
    use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
    use near_sdk::serde::Serializer;

    pub fn serialize<S, K, V>(map: &UnorderedMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Serialize + BorshDeserialize + BorshSerialize,
        V: Serialize + BorshDeserialize + BorshSerialize,
    {
        serializer.collect_seq(map.iter())
    }
}

pub(crate) mod u128_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}
