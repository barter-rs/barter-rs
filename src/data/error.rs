use thiserror::Error;

/// All errors generated in the barter::data module.
#[derive(Error, Debug)]
pub enum DataError {
    #[error("Invalid builder attributes provided")]
    BuilderAttributesInvalid,

    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete,

    #[error("Symbol data iterator does not contain anymore bars")]
    DataIteratorEmpty,
}