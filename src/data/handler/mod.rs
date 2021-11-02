pub mod historic;
pub mod live;

use crate::data::error::DataError;
use crate::data::market::MarketEvent;

/// Determines if a process should continue.
pub trait Continuer {
    /// Return true if a process should continue.
    fn should_continue(&self) -> bool;
}

/// Generates the latest [MarketEvent], acting as the system heartbeat.
pub trait MarketGenerator {
    /// Return the latest [MarketEvent].
    fn generate_market(&mut self) -> Result<MarketEvent, DataError>;
}
