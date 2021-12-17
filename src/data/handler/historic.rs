use crate::data::error::DataError;
use crate::data::handler::{Continuation, Continuer, MarketGenerator};
use crate::data::market::MarketEvent;
use barter_data::model::{Candle, MarketData};
use chrono::Utc;
use uuid::Uuid;

/// Configuration for constructing a [HistoricCandleHandler] via the new() constructor method.
#[derive(Debug)]
pub struct HistoricDataLego<IntoCandles>
where
    IntoCandles: IntoIterator<Item = Candle>
{
    pub exchange: &'static str,
    pub symbol: String,
    pub candles: IntoCandles,
}

#[derive(Debug)]
/// [MarketEvent] data handler that implements [Continuer] & [MarketGenerator]. Simulates a live market
/// feed via drip feeding historical data files as a series of [MarketEvent]s.
pub struct HistoricCandleHandler<Candles>
where
    Candles: Iterator<Item = Candle>
{
    exchange: &'static str,
    symbol: String,
    can_continue: Continuation,
    candles: Candles,
}

impl<Candles> Continuer for HistoricCandleHandler<Candles>
where
    Candles: Iterator<Item = Candle>
{
    fn can_continue(&self) -> &Continuation {
        &self.can_continue
    }
}

impl<Candles> MarketGenerator for HistoricCandleHandler<Candles>
where
    Candles: Iterator<Item = Candle>
{
    fn generate_market(&mut self) -> Option<MarketEvent> {
        match self.candles.into_iter().next() {
            None => {
                self.can_continue = Continuation::Stop;
                None
            },
            Some(candle) => Some(MarketEvent {
                event_type: MarketEvent::EVENT_TYPE,
                trace_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                exchange: self.exchange,
                symbol: self.symbol.clone(),
                data: MarketData::Candle(candle),
            }),
        }
    }
}

impl<Candles> HistoricCandleHandler<Candles>
where
    Candles: Iterator<Item = Candle>
{
    /// Constructs a new [HistoricCandleHandler] component using the provided [HistoricDataLego]
    /// components.
    pub fn new<IntoCandles>(lego: HistoricDataLego<IntoCandles>) -> Self
    where
        IntoCandles: IntoIterator<Item = Candle>
    {
        Self {
            exchange: lego.exchange,
            symbol: lego.symbol,
            can_continue: Continuation::Continue,
            candles: lego.candles.into_iter(),
        }
    }

    // /// Returns a [HistoricCandleHandlerBuilder] instance.
    // pub fn builder<IntoCandles: IntoIterator<Item = Candle> + Default>() -> HistoricCandleHandlerBuilder<IntoCandles> {
    //     HistoricCandleHandlerBuilder::new()
    // }
}

// /// Builder to construct [HistoricCandleHandler] instances.
// #[derive(Debug, Default)]
// pub struct HistoricCandleHandlerBuilder<IntoCandles>
// where
//     IntoCandles: IntoIterator<Item = Candle> + Default,
// {
//     exchange: Option<&'static str>,
//     symbol: Option<String>,
//     candles: Option<IntoCandles>,
// }
//
// impl<IntoCandles> HistoricCandleHandlerBuilder<IntoCandles>
// where
//     IntoCandles: IntoIterator<Item = Candle> + Default,
// {
//     pub fn new() -> Self {
//         Self::default()
//     }
//
//     pub fn symbol(self, value: String) -> Self {
//         Self {
//             symbol: Some(value),
//             ..self
//         }
//     }
//
//     pub fn exchange(self, value: &'static str) -> Self {
//         Self {
//             exchange: Some(value),
//             ..self
//         }
//     }
//
//     pub fn candles(self, value: IntoCandles) -> Self {
//         Self {
//             candles: Some(value),
//             ..self
//         }
//     }
//
//     pub fn build<Candles: Iterator<Item = Candle>>(self) -> Result<HistoricCandleHandler<Candles>, DataError> {
//         let exchange = self.exchange.ok_or(DataError::BuilderIncomplete)?;
//         let symbol = self.symbol.ok_or(DataError::BuilderIncomplete)?;
//         let candles = self.candles.ok_or(DataError::BuilderIncomplete)?.into_iter()
//
//         Ok(HistoricCandleHandler {
//             exchange,
//             symbol,
//             can_continue: Continuation::Continue,
//             candles: candles.into_iter(),
//         })
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_continue_with_symbol_data_remaining() {
        let mut symbol_data_remaining = Vec::with_capacity(2);
        symbol_data_remaining.push(Candle::default());

        let data_handler = HistoricCandleHandler::builder()
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

        let data_handler = HistoricCandleHandler::builder()
            .exchange("Backtest")
            .symbol("DOGE".to_string())
            .candles(symbol_data_remaining.into_iter())
            .build()
            .unwrap();

        let actual_can_continue = data_handler.can_continue();

        assert_eq!(actual_can_continue, &Continuation::Stop);
    }
}
