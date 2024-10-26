use crate::{asset::name::AssetNameInternal, exchange::ExchangeId};
use derive_more::{Constructor, Display, From};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// Defines the Barter [`AssetNameInternal`], used as a `SmolStr` identifier for an [`Asset`]
/// (not unique across exchanges).
pub mod name;

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
    pub name_internal: AssetNameInternal,
    pub name_exchange: SmolStr,
}

impl<S> From<S> for Asset
where
    S: Into<SmolStr>,
{
    fn from(value: S) -> Self {
        Self::new_from_exchange(value)
    }
}

impl Asset {
    pub fn new<Internal, Exchange>(name_internal: Internal, name_exchange: Exchange) -> Self
    where
        Internal: Into<AssetNameInternal>,
        Exchange: Into<SmolStr>,
    {
        Self {
            name_internal: name_internal.into(),
            name_exchange: name_exchange.into(),
        }
    }

    pub fn new_from_exchange<S>(name_exchange: S) -> Self
    where
        S: Into<SmolStr>,
    {
        let name_exchange = name_exchange.into();
        Self {
            name_internal: AssetNameInternal::from(name_exchange.as_str()),
            name_exchange,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Crypto,
    Fiat,
}
