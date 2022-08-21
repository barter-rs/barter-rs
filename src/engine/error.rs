use crate::portfolio::repository::error::RepositoryError;
use thiserror::Error;

/// All errors generated in barter-engine.
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Failed to build struct due to missing attributes: {0}")]
    BuilderIncomplete(&'static str),

    #[error("Failed to interact with repository")]
    RepositoryInteractionError(#[from] RepositoryError),
}
