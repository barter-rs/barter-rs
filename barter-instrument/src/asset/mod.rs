use crate::{asset::symbol::Symbol, exchange::ExchangeId};
use derive_more::{Constructor, Display};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

pub mod symbol;

/// Unique identifier for an [`Asset`].
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub struct AssetId(pub u64);

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Deserialize,
    Serialize,
    Display,
    Constructor,
)]
pub struct AssetIndex(usize);

impl AssetIndex {
    pub fn index(&self) -> usize {
        self.0
    }
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct ExchangeAsset<Asset> {
    pub exchange: ExchangeId,
    pub asset: Asset,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Asset {
    pub kind: AssetKind,
    pub name_internal: Symbol,
    pub name_exchange: SmolStr,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Crypto,
    Fiat,
}