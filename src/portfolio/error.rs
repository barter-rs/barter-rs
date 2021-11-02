use crate::portfolio::repository::error::RepositoryError;
use thiserror::Error;

/// All errors generated in the barter::portfolio module.
#[derive(Error, Debug)]
pub enum PortfolioError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete,

    #[error("Failed to parse Position entry direction due to ambiguous fill quantity & Decision.")]
    ParseEntryDirectionError,

    #[error("Cannot exit Position with an entry decision FillEvent.")]
    CannotEnterPositionWithExitFill,

    #[error("Cannot exit Position with an entry decision FillEvent.")]
    CannotExitPositionWithEntryFill,

    #[error("Failed to interact with repository")]
    RepositoryInteractionError(#[from] RepositoryError),
}
