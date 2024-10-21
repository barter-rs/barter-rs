use derive_more::Display;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub struct AssetId(u64);

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct KeyedAsset<Key, Data> {
    pub key: Key,
    pub asset: Data,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Asset {
    pub kind: AssetKind,
    pub name_internal: SmolStr,
    pub name_exchange: SmolStr,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Crypto,
    Fiat,
}
