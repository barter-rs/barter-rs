use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Represents all possible errors that can occur when searching for indexes in an
/// [`IndexedInstruments`](super::IndexedInstruments) collection.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum IndexError {
    /// Indicates a failure to find an [`ExchangeIndex`](crate::exchange::ExchangeIndex) for a
    /// given exchange identifier.
    ///
    /// Contains a description of the failed lookup attempt.
    #[error("ExchangeIndex: {0}")]
    ExchangeIndex(String),

    /// Indicates a failure to find an [`AssetIndex`](crate::asset::AssetIndex) for a given
    /// asset identifier.
    ///
    /// Contains a description of the failed lookup attempt.
    #[error("AssetIndex: {0}")]
    AssetIndex(String),

    /// Indicates a failure to find an [`InstrumentIndex`](crate::instrument::InstrumentIndex)
    /// for a given instrument identifier.
    ///
    /// Contains a description of the failed lookup attempt.
    #[error("InstrumentIndex: {0}")]
    InstrumentIndex(String),
}
