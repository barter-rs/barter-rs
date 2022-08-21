use crate::portfolio::repository::error::RepositoryError;
use thiserror::Error;

/// All errors generated in the barter::portfolio module.
#[derive(Error, Debug)]
pub enum PortfolioError {
    #[error("Failed to build struct due to missing attributes: {0}")]
    BuilderIncomplete(&'static str),

    #[error("Failed to parse Position entry Side due to ambiguous fill quantity & Decision.")]
    ParseEntrySide,

    #[error("Cannot exit Position with an entry decision FillEvent.")]
    CannotEnterPositionWithExitFill,

    #[error("Cannot exit Position with an entry decision FillEvent.")]
    CannotExitPositionWithEntryFill,

    #[error("Cannot generate PositionExit from Position that has not been exited")]
    PositionExit,

    #[error("Failed to interact with repository")]
    RepositoryInteraction(#[from] RepositoryError),
}
