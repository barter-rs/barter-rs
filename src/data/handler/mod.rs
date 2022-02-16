pub mod historic;
pub mod live;

use std::fmt::{Display, Formatter};
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

impl Display for Continuation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
