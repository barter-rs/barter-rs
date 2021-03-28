use thiserror::Error;

/// All errors generated in the barter::strategy module.
#[derive(Error, Debug)]
pub enum StrategyError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete(),
}

// Todo: Finished data, strategy, need to do execution & portfolio linking.