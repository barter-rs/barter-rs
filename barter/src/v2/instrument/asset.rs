use derive_more::Display;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub struct AssetId(u64);

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Asset {
    pub id: AssetId,
    pub kind: AssetKind,
    pub name_internal: String,
    pub name_exchange: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Crypto,
    Fiat,
}
