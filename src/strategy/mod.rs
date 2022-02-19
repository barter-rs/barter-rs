/// Barter strategy module specific errors.
pub mod error;

///
pub mod strategy;

use crate::Market;
use crate::data::{MarketEvent, MarketMeta};
use crate::strategy::error::StrategyError;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// May generate an advisory [`SignalEvent`] as a result of analysing an input [`MarketEvent`].
pub trait SignalGenerator {
    /// Return Some([`SignalEvent`]), given an input [`MarketEvent`].
    fn generate_signal(&mut self, market: &MarketEvent) -> Option<SignalEvent>;
}

/// Signal data produced by the strategy containing advisory signals for the portfolio to interpret.
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct SignalEvent {
    pub event_type: &'static str,
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub exchange: &'static str,
    pub symbol: String,
    pub signals: HashMap<Decision, SignalStrength>,
    /// Metadata propagated from source MarketEvent
    pub market_meta: MarketMeta,
}

/// Strength of an advisory signal decision produced by the strategy.
pub type SignalStrength = f32;

impl SignalEvent {
    pub const ORGANIC_SIGNAL: &'static str = "Signal";

    /// Returns a [`SignalEventBuilder`] instance.
    pub fn builder() -> SignalEventBuilder {
        SignalEventBuilder::new()
    }
}

/// Describes the type of advisory signal the strategy is endorsing.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Decision {
    Long,
    CloseLong,
    Short,
    CloseShort,
}

impl Default for Decision {
    fn default() -> Self {
        Self::Long
    }
}

impl Decision {
    /// Determines if a [`Decision`] is Long.
    pub fn is_long(&self) -> bool {
        matches!(self, Decision::Long)
    }

    /// Determines if a [`Decision`] is Short.
    pub fn is_short(&self) -> bool {
        matches!(self, Decision::Short)
    }

    /// Determines if a [`Decision`] is an entry (long or short).
    pub fn is_entry(&self) -> bool {
        matches!(self, Decision::Short | Decision::Long)
    }

    /// Determines if a [`Decision`] is an exit (close_long or close_short).
    pub fn is_exit(&self) -> bool {
        matches!(self, Decision::CloseLong | Decision::CloseShort)
    }
}

/// Builder to construct [`SignalEvent`] instances.
#[derive(Debug, Default)]
pub struct SignalEventBuilder {
    pub trace_id: Option<Uuid>,
    pub timestamp: Option<DateTime<Utc>>,
    pub exchange: Option<&'static str>,
    pub symbol: Option<String>,
    pub market_meta: Option<MarketMeta>,
    pub signals: Option<HashMap<Decision, SignalStrength>>,
}

impl SignalEventBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trace_id(self, value: Uuid) -> Self {
        Self {
            trace_id: Some(value),
            ..self
        }
    }

    pub fn timestamp(self, value: DateTime<Utc>) -> Self {
        Self {
            timestamp: Some(value),
            ..self
        }
    }

    pub fn exchange(self, value: &'static str) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }

    pub fn symbol(self, value: String) -> Self {
        Self {
            symbol: Some(value),
            ..self
        }
    }

    pub fn market_meta(self, value: MarketMeta) -> Self {
        Self {
            market_meta: Some(value),
            ..self
        }
    }

    pub fn signals(self, value: HashMap<Decision, SignalStrength>) -> Self {
        Self {
            signals: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<SignalEvent, StrategyError> {
        let trace_id = self.trace_id.ok_or(StrategyError::BuilderIncomplete)?;
        let timestamp = self.timestamp.ok_or(StrategyError::BuilderIncomplete)?;
        let exchange = self.exchange.ok_or(StrategyError::BuilderIncomplete)?;
        let symbol = self.symbol.ok_or(StrategyError::BuilderIncomplete)?;
        let market_meta = self.market_meta.ok_or(StrategyError::BuilderIncomplete)?;
        let signals = self.signals.ok_or(StrategyError::BuilderIncomplete)?;

        Ok(SignalEvent {
            event_type: SignalEvent::ORGANIC_SIGNAL,
            trace_id,
            timestamp,
            exchange,
            symbol,
            market_meta,
            signals,
        })
    }
}

/// Force exit Signal produced after an [`Engine`] receives a [`Command::ExitPosition`](Command)
/// from an external source.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct SignalForceExit {
    pub event_type: &'static str,
    pub timestamp: DateTime<Utc>,
    pub exchange: &'static str,
    pub symbol: String,
}

impl SignalForceExit {
    pub const FORCED_EXIT_SIGNAL: &'static str = "SignalForcedExit";

    /// Constructs a new [`Self`] using the [`Market`] provided.
    pub fn new(market: Market) -> Self {
        Self {
            event_type: SignalForceExit::FORCED_EXIT_SIGNAL,
            timestamp: Utc::now(),
            exchange: market.exchange,
            symbol: market.symbol,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_decision_is_long() {
        let decision = Decision::Long;
        assert_eq!(decision.is_long(), true)
    }

    #[test]
    fn should_return_decision_is_not_long() {
        let decision = Decision::Short;
        assert_eq!(decision.is_long(), false)
    }

    #[test]
    fn should_return_decision_is_short() {
        let decision = Decision::Short;
        assert_eq!(decision.is_short(), true)
    }

    #[test]
    fn should_return_decision_is_not_short() {
        let decision = Decision::Long;
        assert_eq!(decision.is_short(), false)
    }

    #[test]
    fn should_return_decision_is_entry() {
        let decision = Decision::Long;
        assert_eq!(decision.is_entry(), true)
    }

    #[test]
    fn should_return_decision_is_not_entry() {
        let decision = Decision::CloseLong;
        assert_eq!(decision.is_entry(), false)
    }

    #[test]
    fn should_return_decision_is_exit() {
        let decision = Decision::CloseShort;
        assert_eq!(decision.is_exit(), true)
    }

    #[test]
    fn should_return_decision_is_not_exit() {
        let decision = Decision::Long;
        assert_eq!(decision.is_exit(), false)
    }
}