use crate::v2::execution::error::ExecutionError;
use barter_data::error::DataError;
use barter_integration::model::Exchange;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum EngineError {
    #[error("failed to send item over {0} channel due to dropped receiver")]
    RxDropped(&'static str),

    #[error("Engine is setup for exchange {supported}, but received data for {unsupported}")]
    ExchangeUnsupported {
        supported: Exchange,
        unsupported: Exchange,
    },

    #[error("data: {0}")]
    Data(String),

    #[error("execution: {0}")]
    Execution(#[from] ExecutionError),
}

impl EngineError {
    pub fn is_terminal(&self) -> bool {
        false
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for EngineError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::RxDropped(std::any::type_name::<T>())
    }
}

impl From<DataError> for EngineError {
    fn from(value: DataError) -> Self {
        Self::Data(value.to_string())
    }
}
