//! Python bindings for the Barter algorithmic trading framework.
//!
//! Provides access to:
//! - Market data streaming from multiple exchanges
//! - Trading instrument definitions and indexed lookups
//! - Portfolio statistics and performance analysis
//! - Backtesting with historical market data

use pyo3::prelude::*;

mod decimal;
mod runtime;

pub mod data;
pub mod engine;
pub mod execution;
pub mod instrument;
pub mod risk;
pub mod statistics;
pub mod strategy;

/// The native Rust module for barter Python bindings.
#[pymodule]
fn _barter(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register all types and functions
    instrument::register(m)?;
    execution::register(m)?;
    data::register(m)?;
    statistics::register(m)?;
    engine::register(m)?;
    Ok(())
}
