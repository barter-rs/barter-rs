use super::{Decision, Signal, SignalGenerator, SignalStrength};
use crate::data::MarketMeta;
use barter_data::event::{DataKind, MarketEvent};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ta::{indicators::RelativeStrengthIndex, Next};

/// Configuration for constructing a [`RSIStrategy`] via the new() constructor method.
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Config {
    pub rsi_period: usize,
}

#[derive(Clone, Debug)]
/// Example RSI based strategy that implements [`SignalGenerator`].
pub struct RSIStrategy {
    rsi: RelativeStrengthIndex,
}

impl SignalGenerator for RSIStrategy {
    fn generate_signal(&mut self, market: &MarketEvent<DataKind>) -> Option<Signal> {
        // Check if it's a MarketEvent with a candle
        let candle_close = match &market.kind {
            DataKind::Candle(candle) => candle.close,
            _ => return None,
        };

        // Calculate the next RSI value using the new MarketEvent Candle data
        let rsi = self.rsi.next(candle_close);

        // Generate advisory signals map
        let signals = RSIStrategy::generate_signals_map(rsi);

        // If signals map is empty, return no SignalEvent
        if signals.is_empty() {
            return None;
        }

        Some(Signal {
            time: Utc::now(),
            exchange: market.exchange.clone(),
            instrument: market.instrument.clone(),
            market_meta: MarketMeta {
                close: candle_close,
                time: market.exchange_time,
            },
            signals,
        })
    }
}

impl RSIStrategy {
    /// Constructs a new [`RSIStrategy`] component using the provided configuration struct.
    pub fn new(config: Config) -> Self {
        let rsi_indicator = RelativeStrengthIndex::new(config.rsi_period)
            .expect("Failed to construct RSI indicator");

        Self { rsi: rsi_indicator }
    }

    /// Given the latest RSI value for a symbol, generates a map containing the [`SignalStrength`] for
    /// [`Decision`] under consideration.
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

    /// Calculates the [`SignalStrength`] of a particular [`Decision`].
    fn calculate_signal_strength() -> SignalStrength {
        SignalStrength(1.0)
    }
}
