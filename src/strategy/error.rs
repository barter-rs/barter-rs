use thiserror::Error;

/// All errors generated in the barter::strategy module.
#[derive(Error, Copy, Clone, Debug)]
pub enum StrategyError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete,
}
