use crate::data::error::DataError;
use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::data::MarketEvent;
use barter_data::model::{Candle, MarketData};

/// Configuration for constructing a [`HistoricalCandleHandler`] via the new() constructor method.
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct HistoricalDataLego<Candles>
where
    Candles: Iterator<Item = Candle>,
{
    pub exchange: &'static str,
    pub symbol: String,
    pub candles: Candles,
}

#[derive(Clone, PartialEq, PartialOrd, Debug)]
/// [`MarketEvent`] data handler that implements [`Continuer`] & [`MarketGenerator`]. **Simulates**
/// a live market feed via drip feeding historical data files as a series of [`MarketEvent`]s.
pub struct HistoricalCandleHandler<Candles>
where
    Candles: Iterator<Item = Candle>,
{
    exchange: &'static str,
    symbol: String,
    can_continue: Continuation,
    candles: Candles,
}

impl<Candles> Continuer for HistoricalCandleHandler<Candles>
where
    Candles: Iterator<Item = Candle>,
{
    fn can_continue(&self) -> &Continuation {
        &self.can_continue
    }
}

impl<Candles> MarketGenerator for HistoricalCandleHandler<Candles>
where
    Candles: Iterator<Item = Candle>,
{
    fn generate_market(&mut self) -> Option<MarketEvent> {
        // Consume next Candle & generate Some(MarketEvent)
        self.candles
            .next()
            .map(|candle| {
                MarketEvent::new(self.exchange, &self.symbol, MarketData::Candle(candle))
            })
            .or_else(|| {
                self.can_continue = Continuation::Stop;
                None
            })
    }
}

impl<Candles> HistoricalCandleHandler<Candles>
where
    Candles: Iterator<Item = Candle>,
{
    /// Constructs a new [`HistoricalCandleHandler`] component using the provided [`HistoricalDataLego`]
    /// components.
    pub fn new(lego: HistoricalDataLego<Candles>) -> Self {
        Self {
            exchange: lego.exchange,
            symbol: lego.symbol,
            can_continue: Continuation::Continue,
            candles: lego.candles,
        }
    }

    /// Returns a [`HistoricalCandleHandlerBuilder`] instance.
    pub fn builder() -> HistoricalCandleHandlerBuilder<Candles> {
        HistoricalCandleHandlerBuilder::new()
    }
}

/// Builder to construct [`HistoricalCandleHandler`] instances.
#[derive(Debug, Default)]
pub struct HistoricalCandleHandlerBuilder<Candles>
where
    Candles: Iterator<Item = Candle>,
{
    exchange: Option<&'static str>,
    symbol: Option<String>,
    candles: Option<Candles>,
}

impl<Candles> HistoricalCandleHandlerBuilder<Candles>
where
    Candles: Iterator<Item = Candle>,
{
    pub fn new() -> Self {
        Self {
            exchange: None,
            symbol: None,
            candles: None,
        }
    }

    pub fn symbol(self, value: String) -> Self {
        Self {
            symbol: Some(value),
            ..self
        }
    }

    pub fn exchange(self, value: &'static str) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }

    pub fn candles(self, value: Candles) -> Self {
        Self {
            candles: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<HistoricalCandleHandler<Candles>, DataError> {
        Ok(HistoricalCandleHandler {
            exchange: self.exchange.ok_or(DataError::BuilderIncomplete)?,
            symbol: self.symbol.ok_or(DataError::BuilderIncomplete)?,
            can_continue: Continuation::Continue,
            candles: self.candles.ok_or(DataError::BuilderIncomplete)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_data::test_util;

    #[test]
    fn should_continue_with_symbol_data_remaining() {
        let mut symbol_data_remaining = Vec::with_capacity(2);
        symbol_data_remaining.push(test_util::candle());

        let data_handler = HistoricalCandleHandler::builder()
            .exchange("Backtest")
            .symbol("DOGE".to_string())
            .candles(symbol_data_remaining.into_iter())
            .build()
            .unwrap();

        let actual_can_continue = data_handler.can_continue();

        assert_eq!(actual_can_continue, &Continuation::Continue);
    }

    #[test]
    fn should_not_continue_with_no_symbol_data_remaining() {
        let symbol_data_remaining: Vec<Candle> = Vec::with_capacity(2);

        let mut data_handler = HistoricalCandleHandler::builder()
            .exchange("Backtest")
            .symbol("DOGE".to_string())
            .candles(symbol_data_remaining.into_iter())
            .build()
            .unwrap();

        data_handler.generate_market();

        let actual_can_continue = data_handler.can_continue();

        assert_eq!(actual_can_continue, &Continuation::Stop);
    }
}