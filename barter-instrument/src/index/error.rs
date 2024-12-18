use serde::{Deserialize, Serialize};

/// Represents all possible errors that can occur when searching for indexes in an
/// [`IndexedInstruments`](super::IndexedInstruments) collection.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum IndexError {
    /// Indicates a failure to find an [`ExchangeIndex`](crate::exchange::ExchangeIndex) for a
    /// given exchange identifier.
    ///
    /// Contains a description of the failed lookup attempt.
    ExchangeIndex(String),

    /// Indicates a failure to find an [`AssetIndex`](crate::asset::AssetIndex) for a given
    /// asset identifier.
    ///
    /// Contains a description of the failed lookup attempt.
    AssetIndex(String),

    /// Indicates a failure to find an [`InstrumentIndex`](crate::instrument::InstrumentIndex)
    /// for a given instrument identifier.
    ///
    /// Contains a description of the failed lookup attempt.
    InstrumentIndex(String),
}
