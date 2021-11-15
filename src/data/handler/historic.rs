use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::data::market::MarketEvent;
use crate::data::error::DataError;
use barter_data::model::{Candle, MarketData};
use chrono::Utc;
use std::vec::IntoIter;
use uuid::Uuid;

/// Configuration for constructing a [HistoricDataHandler] via the new() constructor method.
#[derive(Debug)]
pub struct HistoricDataLego {
    pub exchange: String,
    pub symbol: String,
    pub candle_iterator: IntoIter<Candle>,
}

/// [MarketEvent] data handler that implements [Continuer] & [MarketGenerator]. Simulates a live market
/// feed via drip feeding historical data files as a series of [MarketEvent]s.
pub struct HistoricCandleHandler {
    exchange: String,
    symbol: String,
    candle_iterator: IntoIter<Candle>,
}

impl Continuer for HistoricCandleHandler {
    fn can_continue(&self) -> &Continuation {
        match self.candle_iterator.len() != 0 {
            true => &Continuation::Continue,
            false => &Continuation::Stop
        }
    }
}

impl MarketGenerator for HistoricCandleHandler {
    fn generate_market(&mut self) -> Option<MarketEvent> {
        match self.candle_iterator.next() {
            None => None,
            Some(candle) => Some(
                MarketEvent {
                    event_type: MarketEvent::EVENT_TYPE,
                    trace_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    exchange: self.exchange.clone(),
                    symbol: self.symbol.clone(),
                    data: MarketData::Candle(candle)
                }
            )
        }
    }
}

impl HistoricCandleHandler {
    /// Constructs a new [HistoricDataHandler] component using the provided [HistoricDataLego]
    /// components.
    pub fn new(lego: HistoricDataLego) -> Self {
        Self {
            exchange: lego.exchange,
            symbol: lego.symbol,
            candle_iterator: lego.candle_iterator,
        }
    }

    /// Returns a [HistoricDataHandlerBuilder] instance.
    pub fn builder() -> HistoricDataHandlerBuilder {
        HistoricDataHandlerBuilder::new()
    }
}

/// Builder to construct [HistoricDataHandler] instances.
#[derive(Debug, Default)]
pub struct HistoricDataHandlerBuilder {
    exchange: Option<String>,
    symbol: Option<String>,
    candle_iterator: Option<IntoIter<Candle>>,
}

impl HistoricDataHandlerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn symbol(self, value: String) -> Self {
        Self {
            symbol: Some(value),
            ..self
        }
    }

    pub fn exchange(self, value: String) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }

    pub fn candle_iterator(self, value: IntoIter<Candle>) -> Self {
        Self {
            candle_iterator: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<HistoricCandleHandler, DataError> {
        let exchange = self.exchange.ok_or(DataError::BuilderIncomplete)?;
        let symbol = self.symbol.ok_or(DataError::BuilderIncomplete)?;
        let candle_iterator = self.candle_iterator.ok_or(DataError::BuilderIncomplete)?;

        Ok(HistoricCandleHandler {
            exchange,
            symbol,
            candle_iterator,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_continue_with_symbol_data_remaining() {
        let mut symbol_data_remaining = Vec::with_capacity(2);
        symbol_data_remaining.push(Candle::default());

        let data_handler = HistoricCandleHandler::builder()
            .exchange("BACKTEST".to_string())
            .symbol("DOGE".to_string())
            .candle_iterator(symbol_data_remaining.into_iter())
            .build()
            .unwrap();

        let actual_can_continue = data_handler.can_continue();

        assert_eq!(actual_can_continue, &Continuation::Continue);
    }

    #[test]
    fn should_not_continue_with_no_symbol_data_remaining() {
        let symbol_data_remaining: Vec<Candle> = Vec::with_capacity(2);

        let data_handler = HistoricCandleHandler::builder()
            .exchange("BACKTEST".to_string())
            .symbol("DOGE".to_string())
            .candle_iterator(symbol_data_remaining.into_iter())
            .build()
            .unwrap();

        let actual_can_continue = data_handler.can_continue();

        assert_eq!(actual_can_continue, &Continuation::Stop);
    }
}
