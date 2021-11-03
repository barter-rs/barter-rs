pub mod historic;
pub mod live;

use crate::data::market::MarketEvent;
use serde::{Deserialize, Serialize};

/// Determines if a process should continue.
pub trait Continuer {
    /// Return true if a process should continue.
    fn should_continue(&mut self) -> Continuation;
}

/// Generates the latest [MarketEvent], acting as the system heartbeat.
pub trait MarketGenerator {
    /// Return the latest [MarketEvent].
    fn generate_market(&mut self) -> Option<MarketEvent>;
}

/// Returned by a [Continuer] to communicate if a process should continue.
#[derive(Debug, Deserialize, Serialize)]
pub enum Continuation {
    Continue,
    Stop,
}
