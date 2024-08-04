use crate::v2::execution::error::IndexedExecutionError;
use barter_data::error::DataError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum BarterError {
    #[error("IndexError: {0}")]
    IndexError(#[from] IndexError),

    #[error("KeyError: {0}")]
    Key(#[from] KeyError),

    #[error("ExecutionBuilder: {0}")]
    ExecutionBuilder(String),

    #[error("ExchangeManager dropped it's ExecutionRequest receiver")]
    ExecutionRxDropped(#[from] RxDropped),

    #[error("market data: {0}")]
    MarketData(#[from] DataError),

    #[error("execution: {0}")]
    Execution(#[from] IndexedExecutionError),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum KeyError {
    #[error("ExchangeId: {0}")]
    ExchangeId(String),

    #[error("AssetKey: {0}")]
    AssetKey(String),

    #[error("InstrumentKey: {0}")]
    InstrumentKey(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum IndexError {
    #[error("ExchangeIndex: {0}")]
    ExchangeIndex(String),

    #[error("AssetIndex: {0}")]
    AssetIndex(String),

    #[error("InstrumentIndex: {0}")]
    InstrumentIndex(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
#[error("RxDropped")]
pub struct RxDropped;

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for RxDropped {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for BarterError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::ExecutionRxDropped(RxDropped)
    }
}
