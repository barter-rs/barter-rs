use crate::portfolio::repository::error::RepositoryError;
use thiserror::Error;

/// All errors generated in barter-engine.
#[derive(Error, Copy, Clone, Debug)]
pub enum EngineError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete,

    #[error("Failed to interact with repository")]
    RepositoryInteractionError(#[from] RepositoryError),
}
