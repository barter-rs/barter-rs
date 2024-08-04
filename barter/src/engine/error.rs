use barter_instrument::index::error::IndexError;
use barter_integration::Unrecoverable;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Represents possible errors that can occur in the [`Engine`](super::Engine).
///
/// A distinction is made between a recoverable and unrecoverable error:
/// - Recoverable errors do not result in the termination the `Engine`.
/// - Unrecoverable errors result in the graceful termination of the `Engine`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum EngineError {
    #[error("recoverable error: {0}")]
    Recoverable(#[from] RecoverableEngineError),

    #[error("unrecoverable error: {0}")]
    Unrecoverable(#[from] UnrecoverableEngineError),
}

/// Represents temporary error conditions that the [`Engine`](super::Engine) can recover from.
///
/// These errors typically represent transient issues like network problems.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum RecoverableEngineError {
    #[error("ExecutionRequest channel unhealthy: {0}")]
    ExecutionChannelUnhealthy(String),
}

/// Represents fatal error conditions that the [`Engine`](super::Engine) cannot recover from.
///
/// These errors typically represent fundamental issues that require human
/// intervention or a system restart to resolve.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum UnrecoverableEngineError {
    #[error("IndexError: {0}")]
    IndexError(#[from] IndexError),

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
