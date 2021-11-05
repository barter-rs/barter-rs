pub mod error;
pub mod signal;
pub mod strategy;

use crate::data::market::MarketEvent;
use crate::strategy::signal::SignalEvent;

/// May generate an advisory [SignalEvent] as a result of analysing an input [MarketEvent].
pub trait SignalGenerator {
    /// Return Some([SignalEvent]), given an input [MarketEvent].
    fn generate_signal(&mut self, market: &MarketEvent) -> Option<SignalEvent>;
}
