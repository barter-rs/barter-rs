use barter_execution::error::IndexedClientError;
use barter_instrument::index::error::IndexError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Error)]
pub enum ExecutionError {
    #[error("ExecutionManager config invalid: {0}")]
    Config(String),

    #[error("IndexError: {0}")]
    Index(#[from] IndexError),

    #[error("{0}")]
    Client(#[from] IndexedClientError),
}
