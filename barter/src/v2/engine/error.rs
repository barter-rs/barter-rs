use crate::v2::error::{IndexError, KeyError};
use barter_integration::Unrecoverable;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum EngineError {
    #[error("recoverable error: {0}")]
    Recoverable(#[from] RecoverableEngineError),

    #[error("unrecoverable error: {0}")]
    Unrecoverable(#[from] UnrecoverableEngineError),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum RecoverableEngineError {
    #[error("ExecutionRequest channel unhealthy: {0}")]
    ExecutionChannelUnhealthy(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]

pub enum UnrecoverableEngineError {
    #[error("IndexError: {0}")]
    IndexError(#[from] IndexError),

    #[error("KeyError: {0}")]
    Key(#[from] KeyError),

    #[error("ExecutionRequest channel terminated: {0}")]
    ExecutionChannelTerminated(String),

    #[error("{0}")]
    Custom(String),
}

impl Unrecoverable for EngineError {
    fn is_unrecoverable(&self) -> bool {
        matches!(self, EngineError::Unrecoverable(_))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
#[error("ExecutionRxDropped")]
pub struct ExecutionRxDropped;
