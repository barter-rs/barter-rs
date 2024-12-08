use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum IndexError {
    #[error("ExchangeIndex: {0}")]
    ExchangeIndex(String),

    #[error("AssetIndex: {0}")]
    AssetIndex(String),

    #[error("InstrumentIndex: {0}")]
    InstrumentIndex(String),
}
