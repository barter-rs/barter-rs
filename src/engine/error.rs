use thiserror::Error;

/// All errors generated in barter-engine.
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete,
}
