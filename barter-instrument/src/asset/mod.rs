use crate::{
    Keyed,
    asset::name::{AssetNameExchange, AssetNameInternal},
    exchange::ExchangeId,
};
use derive_more::{Constructor, Display};
use serde::{Deserialize, Serialize};

/// Defines the [`AssetNameInternal`] and [`AssetNameExchange`] types, used as `SmolStr`
/// identifiers for an [`Asset`].
pub mod name;

/// Unique identifier for an [`Asset`].
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub struct AssetId(pub u64);

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct AssetIndex(pub usize);

impl AssetIndex {
    pub fn index(&self) -> usize {
        self.0
    }
}

impl std::fmt::Display for AssetIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AssetIndex({})", self.0)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct ExchangeAsset<Asset> {
    pub exchange: ExchangeId,
    pub asset: Asset,
}

impl<Asset> ExchangeAsset<Asset> {
    pub fn new<A>(exchange: ExchangeId, asset: A) -> Self
    where
        A: Into<Asset>,
    {
        Self {
            exchange,
            asset: asset.into(),
        }
    }
}

impl<Ass, Asset, T> From<(ExchangeId, Ass, T)> for Keyed<ExchangeAsset<Asset>, T>
where
    Ass: Into<Asset>,
{
    fn from((exchange, asset, value): (ExchangeId, Ass, T)) -> Self {
        Self {
            key: ExchangeAsset::new(exchange, asset),
            value,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Asset {
    pub name_internal: AssetNameInternal,
    pub name_exchange: AssetNameExchange,
}

impl<S> From<S> for Asset
where
    S: Into<AssetNameExchange>,
{
    fn from(value: S) -> Self {
        Self::new_from_exchange(value)
    }
}

impl Asset {
    pub fn new<Internal, Exchange>(name_internal: Internal, name_exchange: Exchange) -> Self
    where
        Internal: Into<AssetNameInternal>,
        Exchange: Into<AssetNameExchange>,
    {
        Self {
            name_internal: name_internal.into(),
            name_exchange: name_exchange.into(),
        }
    }

    pub fn new_from_exchange<S>(name_exchange: S) -> Self
    where
        S: Into<AssetNameExchange>,
    {
        let name_exchange = name_exchange.into();
        Self {
            name_internal: AssetNameInternal::from(name_exchange.name().clone()),
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

impl From<Asset> for AssetNameInternal {
    fn from(value: Asset) -> Self {
        value.name_internal
    }
}

/// Special type that represents a "base" [`Asset`].
///
/// Examples: <br>
/// a) Instrument = btc_usdt_spot, [`BaseAsset`] => btc <br>
/// b) Instrument = eth_btc_spot, [`BaseAsset`] => eth
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display)]
pub struct BaseAsset;

/// Special type that represents a "quote" [`Asset`].
///
/// Examples: <br>
/// a) Instrument = btc_usdt_spot, [`QuoteAsset`] => usdt <br>
/// b) Instrument = eth_btc_spot, [`QuoteAsset`] => btc
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display)]
pub struct QuoteAsset;
