use thiserror::Error;

/// All errors generated in the barter::strategy module.
#[derive(Error, Debug)]
pub enum StrategyError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete,
}
