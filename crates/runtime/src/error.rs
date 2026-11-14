use crate::execution::error::ExecutionError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use trading_bot_instrument::index::error::IndexError;
use trading_bot_market::error::DataError;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum TradingBotError {
    #[error("IndexError: {0}")]
    IndexError(#[from] IndexError),

    #[error("ExecutionBuilder: {0}")]
    ExecutionBuilder(String),

    #[error("ExchangeManager dropped it's ExecutionRequest receiver")]
    ExecutionRxDropped(#[from] RxDropped),

    #[error("market data: {0}")]
    MarketData(#[from] DataError),

    #[error("execution: {0}")]
    Execution(#[from] ExecutionError),

    #[error("JoinError: {0}")]
    JoinError(String),
}
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
#[error("RxDropped")]
pub struct RxDropped;

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for RxDropped {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for TradingBotError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::ExecutionRxDropped(RxDropped)
    }
}

impl From<tokio::task::JoinError> for TradingBotError {
    fn from(value: tokio::task::JoinError) -> Self {
        Self::JoinError(format!("{value:?}"))
    }
}
