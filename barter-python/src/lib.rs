//! Python bindings for the Barter algorithmic trading framework.
//!
//! Provides access to:
//! - Market data streaming from multiple exchanges
//! - Trading instrument definitions and indexed lookups
//! - Portfolio statistics and performance analysis
//! - Backtesting with historical market data
//! - Custom strategy and risk management callbacks

use pyo3::prelude::*;

mod decimal;
mod runtime;

pub mod account;
pub mod data;
pub mod engine;
pub mod execution;
pub mod global;
pub mod instrument;
pub mod instrument_data;
pub mod order;
pub mod orderbook;
pub mod risk;
pub mod state;
pub mod statistics;
pub mod strategy;

/// The native Rust module for barter Python bindings.
#[pymodule]
fn _barter(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register all types and functions
    account::register(m)?;
    instrument::register(m)?;
    execution::register(m)?;
    order::register(m)?;
    orderbook::register(m)?;
    state::register(m)?;
    data::register(m)?;
    statistics::register(m)?;
    engine::register(m)?;
    Ok(())
}
