use thiserror::Error;

/// All errors generated in the barter::execution module.
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete,
}
