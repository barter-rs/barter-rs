use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::strategy::error::StrategyError::BuilderIncomplete;
use crate::strategy::error::StrategyError;
use crate::data::market::MarketMeta;

/// Signal data produced by the strategy containing advisory signals for the portfolio to interpret.
#[derive(Debug, Serialize, Deserialize)]
pub struct SignalEvent {
    pub event_type: &'static str,
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub exchange: String,
    pub symbol: String,
    /// Metadata propagated from source MarketEvent
    pub market_meta: MarketMeta,
    pub signals: HashMap<Decision, SignalStrength>,
}

/// Strength of an advisory signal decision produced by the strategy.
pub type SignalStrength = f32;

impl Default for SignalEvent {
    fn default() -> Self {
        Self {
            event_type: SignalEvent::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: String::from("BINANCE"),
            symbol: String::from("ETH-USD"),
            market_meta: MarketMeta::default(),
            signals: Default::default(),
        }
    }
}

impl SignalEvent {
    pub const EVENT_TYPE: &'static str = "SignalEvent";

    /// Returns a [SignalEventBuilder] instance.
    pub fn builder() -> SignalEventBuilder {
        SignalEventBuilder::new()
    }
}

/// Describes the type of advisory signal the strategy is endorsing.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Determines if a [Decision] is Long.
    pub fn is_long(&self) -> bool {
        matches!(self, Decision::Long)
    }

    /// Determines if a [Decision] is Short.
    pub fn is_short(&self) -> bool {
        matches!(self, Decision::Short)
    }

    /// Determines if a [Decision] is an entry (long or short).
    pub fn is_entry(&self) -> bool {
        matches!(self, Decision::Short | Decision::Long)
    }

    /// Determines if a [Decision] is an exit (close_long or close_short).
    pub fn is_exit(&self) -> bool {
        matches!(self, Decision::CloseLong | Decision::CloseShort)
    }
}

/// Builder to construct [SignalEvent] instances.
#[derive(Debug, Default)]
pub struct SignalEventBuilder {
    pub trace_id: Option<Uuid>,
    pub timestamp: Option<DateTime<Utc>>,
    pub exchange: Option<String>,
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

    pub fn exchange(self, value: String) -> Self {
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
        if let (
            Some(trace_id),
            Some(timestamp),
            Some(exchange),
            Some(symbol),
            Some(market_meta),
            Some(signals)
        ) = (
            self.trace_id,
            self.timestamp,
            self.exchange,
            self.symbol,
            self.market_meta,
            self.signals
        ) {
            Ok(SignalEvent {
                event_type: SignalEvent::EVENT_TYPE,
                trace_id,
                timestamp,
                exchange,
                symbol,
                market_meta,
                signals,
            })
        } else {
            Err(BuilderIncomplete)
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