use derive_more::Constructor;
use serde::{Deserialize, Serialize};

/// Defines a global [`ExchangeId`] enum covering all exchanges.
pub mod exchange;

/// [`Asset`](asset::Asset) related data structures.
///
/// eg/ `AssetKind`, `Symbol`, etc.
pub mod asset;

/// [`Instrument`](instrument::Instrument) related data structures.
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
