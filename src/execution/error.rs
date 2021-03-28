use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Failed to build struct due to incomplete attributes provided")]
    BuilderIncomplete(),
}