use thiserror::Error;

/// All errors generated in the barter::portfolio::repository module.
#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Failed to deserialize/serialize JSON due to: {0}")]
    JsonSerDeError(#[from] serde_json::Error),

    #[error("Failed to write data to the repository")]
    WriteError,

    #[error("Failed to read data from the repository")]
    ReadError,

    #[error("Failed to delete data from the repository")]
    DeleteError,

    #[error("Failed to retrieve expected data due to it not being present")]
    ExpectedDataNotPresentError,
}
