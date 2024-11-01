use derive_more::Constructor;
use serde::{Deserialize, Serialize};

/// Defines a global [`ExchangeId`] enum covering all exchanges.
pub mod exchange;

/// [`Asset`](asset::Asset) related data structures.
///
/// eg/ `AssetKind`, `AssetNameInternal`, etc.
pub mod asset;

/// [`Instrument`](market_data::MarketDataInstrument) related data structures.
///
/// eg/ `InstrumentKind`, `OptionContract``, etc.
pub mod instrument;

pub mod market;

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Keyed<Key, Value> {
    pub key: Key,
    pub value: Value,
}

impl<Key, Value> AsRef<Value> for Keyed<Key, Value> {
    fn as_ref(&self) -> &Value {
        &self.value
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Underlying<AssetKey> {
    pub base: AssetKey,
    pub quote: AssetKey,
}

impl<AssetKey> Underlying<AssetKey> {
    pub fn new<A>(base: A, quote: A) -> Self
    where
        A: Into<AssetKey>,
    {
        Self {
            base: base.into(),
            quote: quote.into(),
        }
    }
}
