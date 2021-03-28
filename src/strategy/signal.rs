use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::strategy::error::StrategyError::BuilderIncomplete;
use crate::strategy::error::StrategyError;

/// Signal data produced by the strategy containing advisory signals for the portfolio to interpret.
#[derive(Debug, Serialize, Deserialize)]
pub struct SignalEvent {
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub exchange: String,
    pub symbol: String,
    pub close: f64,
    pub signals: HashMap<Decision, SignalStrength>,
}

/// Strength of an advisory signal decision produced by the strategy.
pub type SignalStrength = f32;

impl Default for SignalEvent {
    fn default() -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: String::from("BINANCE"),
            symbol: String::from("ETH-USD"),
            close: 1050.0,
            signals: Default::default(),
        }
    }
}

impl SignalEvent {
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
    /// Determines if a [Decision] is (long or close_long).
    pub fn is_long_or_close_long(&self) -> bool {
        match self {
            Decision::Long => true,
            Decision::CloseLong => true,
            _ => false,
        }
    }

    /// Determines if a [Decision] is (short or close_short).
    pub fn is_short_or_close_short(&self) -> bool {
        match self {
            Decision::Short => true,
            Decision::CloseShort => true,
            _ => false,
        }
    }

    /// Determines if a [Decision] is an exit (close_long or close_short).
    pub fn is_exit(&self) -> bool {
        match self {
            Decision::CloseLong => true,
            Decision::CloseShort => true,
            _ => false,
        }
    }
}

/// Builder to construct [SignalEvent] instances.
pub struct SignalEventBuilder {
    pub trace_id: Option<Uuid>,
    pub timestamp: Option<DateTime<Utc>>,
    pub exchange: Option<String>,
    pub symbol: Option<String>,
    pub close: Option<f64>,
    pub signals: Option<HashMap<Decision, SignalStrength>>,
}

impl SignalEventBuilder {
    pub fn new() -> Self {
        Self {
            trace_id: None,
            timestamp: None,
            exchange: None,
            symbol: None,
            close: None,
            signals: None,
        }
    }

    pub fn trace_id(mut self, value: Uuid) -> Self {
        self.trace_id = Some(value);
        self
    }

    pub fn timestamp(mut self, value: DateTime<Utc>) -> Self {
        self.timestamp = Some(value);
        self
    }

    pub fn exchange(mut self, value: String) -> Self {
        self.exchange = Some(value);
        self
    }

    pub fn symbol(mut self, value: String) -> Self {
        self.symbol = Some(value);
        self
    }

    pub fn close(mut self, value: f64) -> Self {
        self.close = Some(value);
        self
    }

    pub fn signals(mut self, value: HashMap<Decision, SignalStrength>) -> Self {
        self.signals = Some(value);
        self
    }

    pub fn build(self) -> Result<SignalEvent, StrategyError> {
        if let (
            Some(trace_id),
            Some(timestamp),
            Some(exchange),
            Some(symbol),
            Some(close),
            Some(signals)
        ) = (
            self.trace_id,
            self.timestamp,
            self.exchange,
            self.symbol,
            self.close,
            self.signals
        ) {
            Ok(SignalEvent {
                trace_id,
                timestamp,
                exchange,
                symbol,
                close,
                signals,
            })
        } else {
            Err(BuilderIncomplete())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_decision_is_long_or_close_long() {
        let decision = Decision::Long;
        assert_eq!(decision.is_long_or_close_long(), true)
    }

    #[test]
    fn should_return_decision_is_not_long_or_close_long() {
        let decision = Decision::Short;
        assert_eq!(decision.is_long_or_close_long(), false)
    }

    #[test]
    fn should_return_decision_is_short_or_close_short() {
        let decision = Decision::CloseShort;
        assert_eq!(decision.is_short_or_close_short(), true)
    }

    #[test]
    fn should_return_decision_is_not_short_or_close_short() {
        let decision = Decision::Long;
        assert_eq!(decision.is_short_or_close_short(), false)
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