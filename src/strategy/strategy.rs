use crate::data::market::MarketEvent;
use crate::strategy::signal::{SignalEvent, Decision, SignalStrength};
use crate::strategy::error::StrategyError;
use ta::indicators::RelativeStrengthIndex;
use ta::Next;
use chrono::Utc;
use serde::Deserialize;
use crate::strategy::error::StrategyError::BuilderIncomplete;
use std::collections::HashMap;

/// May generate an advisory [SignalEvent] as a result of analysing an input [MarketEvent].
pub trait SignalGenerator {
    /// Return Some([SignalEvent]), given an input [MarketEvent].
    fn generate_signal(&mut self, market: &MarketEvent) -> Result<Option<SignalEvent>, StrategyError>;
}

/// Configuration for constructing a [RSIStrategy] via the new() constructor method.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub rsi_period: usize,
}

/// Example RSI based strategy that implements [SignalGenerator].
pub struct RSIStrategy {
    rsi: RelativeStrengthIndex,
}

impl SignalGenerator for RSIStrategy {
    fn generate_signal(&mut self, market: &MarketEvent) -> Result<Option<SignalEvent>, StrategyError> {
        // Calculate the next RSI value using the new MarketEvent.Bar data
        let rsi = self.rsi.next(&market.bar);

        // Generate advisory signals map
        let signals = RSIStrategy::generate_signals_map(rsi);

        // If signals map is empty, return no SignalEvent
        if signals.is_empty() {
            return Ok(None)
        }

        Ok(Some(SignalEvent {
            trace_id: market.trace_id,
            timestamp: Utc::now(),
            exchange: market.exchange.clone(),
            symbol: market.symbol.clone(),
            close: market.bar.close,
            signals
        }))
    }
}

impl RSIStrategy {
    /// Constructs a new [RSIStrategy] component using the provided configuration struct.
    pub fn new(config: &Config) -> Self {
        let rsi_indicator = RelativeStrengthIndex::new(config.rsi_period)
            .expect("Failed to construct RSI indicator");

        Self {
            rsi: rsi_indicator
        }
    }

    /// Returns a [RSIStrategyBuilder] instance.
    pub fn builder() -> RSIStrategyBuilder {
        RSIStrategyBuilder::new()
    }

    /// Given the latest RSI value for a symbol, generates a map containing the [SignalStrength] for
    /// [Decision] under consideration.
    fn generate_signals_map(rsi: f64) -> HashMap<Decision, SignalStrength> {
        let mut signals = HashMap::with_capacity(4);
        if rsi < 40.0 {
            signals.insert(Decision::Long, RSIStrategy::calculate_signal_strength());
        }
        if rsi > 60.0 {
            signals.insert(
                Decision::CloseLong,
                RSIStrategy::calculate_signal_strength(),
            );
        }
        if rsi > 60.0 {
            signals.insert(Decision::Short, RSIStrategy::calculate_signal_strength());
        }
        if rsi < 40.0 {
            signals.insert(
                Decision::CloseShort,
                RSIStrategy::calculate_signal_strength(),
            );
        }
        signals
    }

    /// Calculates the [SignalStrength] of a particular [Decision].
    fn calculate_signal_strength() -> f32 {
        1.0
    }
}

/// Builder to construct [RSIStrategy] instances.
pub struct RSIStrategyBuilder {
    rsi: Option<RelativeStrengthIndex>,
}

impl RSIStrategyBuilder {
    pub fn new() -> Self {
        Self { rsi: None }
    }

    pub fn rsi(mut self, value: RelativeStrengthIndex) -> Self {
        self.rsi = Some(value);
        self
    }

    pub fn build(self) -> Result<RSIStrategy, StrategyError> {
        if let Some(rsi) = self.rsi {
            Ok(RSIStrategy { rsi })
        } else {
            Err(BuilderIncomplete())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_generate_signals_map_containing_long_and_close_short_decision() {
        let input_rsi = 20.0;

        let actual_signals = RSIStrategy::generate_signals_map(input_rsi);

        assert!(
            actual_signals.contains_key(&Decision::Long) &&
                actual_signals.contains_key(&Decision::CloseShort)
        )
    }
}