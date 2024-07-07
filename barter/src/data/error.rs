use barter_integration::error::SocketError;
use thiserror::Error;

/// All errors generated in the barter::data module.
#[derive(Error, Debug)]
pub enum DataError {
    #[error("Invalid builder attributes provided")]
    BuilderAttributesInvalid,

    #[error("Failed to build struct due to missing attributes: {0}")]
    BuilderIncomplete(&'static str),

    #[error("Socket: {0}")]
    Socket(#[from] SocketError),

    #[error("Barter-Data: {0}")]
    Data(#[from] barter_data::error::DataError),
}
