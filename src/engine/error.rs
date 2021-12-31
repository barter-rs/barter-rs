use thiserror::Error;
use crate::portfolio::repository::error::RepositoryError;

/// All errors generated in barter-engine.
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete,

    #[error("Failed to interact with repository")]
    RepositoryInteractionError(#[from] RepositoryError),
}
