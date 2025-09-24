use thiserror::Error;

#[derive(Error, Debug)]
pub enum StrategyError {
    #[error("Data collection error: {0}")]
    DataCollection(String),

    #[error("Signal processing error: {0}")]
    SignalProcessing(String),

    #[error("Model inference error: {0}")]
    ModelInference(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Queue error: {0}")]
    Queue(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Decimal conversion error: {0}")]
    Decimal(#[from] rust_decimal::Error),

    #[error("Integration error: {0}")]
    Integration(#[from] barter_integration::error::Error),
}

pub type Result<T> = std::result::Result<T, StrategyError>;