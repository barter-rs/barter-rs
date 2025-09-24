pub mod signal;
pub mod processor;
pub mod judgment;
pub mod action;
pub mod execution;
pub mod queue;
pub mod model;
pub mod backtest;
pub mod config;
pub mod error;

pub use signal::SignalCollector;
pub use processor::SignalProcessor;
pub use judgment::SignalJudgment;
pub use action::StrategyAction;
pub use execution::StrategyExecution;
pub use error::{StrategyError, Result};