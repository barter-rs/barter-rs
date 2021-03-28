use crate::data::market::MarketEvent;
use crate::strategy::signal::SignalEvent;
use crate::strategy::error::StrategyError;
use ta::indicators::RelativeStrengthIndex;
use ta::Next;
use chrono::Utc;

/// May generate an advisory SignalEvent as a result of analysing an input MarketEvent.
pub trait SignalGenerator {
    /// May return a SignalEvent, given an input MarketEvent.
    fn generate_signal(&mut self, market: &MarketEvent) -> Result<Option<SignalEvent>, StrategyError>;
}

/// Configuration for constructing a RSIStrategy via the new() constructor method.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub rsi_period: usize,
}

/// Example RSI based strategy that implements SignalGenerator.
pub struct RSIStrategy {
    rsi: RelativeStrengthIndex,
}

impl SignalGenerator for RSIStrategy {
    fn generate_signal(&mut self, market: &MarketEvent) -> Result<Option<SignalEvent>, StrategyError> {
        // Calculate the next RSI value using the new MarketEvent.Bar data
        let rsi = self.rsi.next(&market.bar);

        // Generate advisory signals map
        let signals = self.generate_signals_map(rsi);

        // If signals in map, return Some(SignalEvent)
        if signals.is_empty() {
            Ok(Some(SignalEvent::builder()
                .trace_id(market.trace_id)
                .timestamp(Utc::now())
                .exchange(market.exchange.clone())
                .symbol(market.symbol.clone())
                .close(market.bar.close)
                .signals(signals)
                .build()?))
        }

        Ok(None)
    }
}