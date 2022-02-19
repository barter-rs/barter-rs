/// Historical data handlers for backtesting.
pub mod historic;

/// Live data handlers for dry-trading & live-trading.
pub mod live;

use crate::data::market::MarketEvent;
use serde::{Deserialize, Serialize};

/// Determines if a process can continue.
pub trait Continuer {
    /// Returns a [Continuation] to communicate if a process can continue.
    fn can_continue(&self) -> &Continuation;
}

/// Generates the latest [MarketEvent], acting as the system heartbeat.
pub trait MarketGenerator {
    /// Return the latest [MarketEvent].
    fn generate_market(&mut self) -> Option<MarketEvent>;
}

/// Returned by a [Continuer] to communicate if a process should continue.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Continuation {
    Continue,
    Stop,
}
