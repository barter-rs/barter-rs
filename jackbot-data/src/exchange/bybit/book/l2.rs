// Bybit Order Book L2: Channel, message, normalization, and tests
// See Bybit WebSocket API: https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook

use crate::books::{Canonicalizer, Level, OrderBook};
use crate::event::{MarketEvent, MarketIter};
use crate::exchange::bybit::message::BybitPayload;
use crate::subscription::book::OrderBookEvent;
use chrono::{DateTime, Utc};
use jackbot_instrument::exchange::ExchangeId;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};

/// Terse type alias for a Bybit L2 order book WebSocket message
pub type BybitOrderBookL2 = BybitPayload<BybitOrderBookL2Data>;

/// Bybit L2 order book data (array of levels)
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitOrderBookL2Data {
    #[serde(rename = "s")]
    pub market: String,
    #[serde(default)]
    pub bids: Vec<BybitOrderBookLevel>,
    #[serde(default)]
    pub asks: Vec<BybitOrderBookLevel>,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitOrderBookLevel {
    #[serde(alias = "price", deserialize_with = "jackbot_integration::de::de_str")]
    pub price: f64,
    #[serde(alias = "size", deserialize_with = "jackbot_integration::de::de_str")]
    pub size: f64,
}

impl Canonicalizer for BybitOrderBookL2Data {
    fn canonicalize(&self, timestamp: DateTime<Utc>) -> OrderBook {
        use rust_decimal::Decimal;
        OrderBook::new(
            0, // Bybit L2 doesn't provide sequence
            Some(timestamp),
            self.bids.iter().map(|l| {
                Level::new(
                    Decimal::from_f64(l.price).unwrap(),
                    Decimal::from_f64(l.size).unwrap(),
                )
            }),
            self.asks.iter().map(|l| {
                Level::new(
                    Decimal::from_f64(l.price).unwrap(),
                    Decimal::from_f64(l.size).unwrap(),
                )
            }),
        )
    }
}

// For backward compatibility
impl BybitOrderBookL2Data {
    pub fn normalize(&self, time: DateTime<Utc>) -> OrderBook {
        self.canonicalize(time)
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, OrderBookEvent)>
    for MarketEvent<InstrumentKey, OrderBookEvent>
{
    fn from((exchange_id, instrument, event): (ExchangeId, InstrumentKey, OrderBookEvent)) -> Self {
        Self {
            time_exchange: Utc::now(),
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: event,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_bybit_l2_deserialize_and_canonicalize() {
        let json = r#"{
            "topic": "orderbook.BTCUSDT",
            "type": "snapshot",
            "ts": 1672304486868,
            "data": {
                "s": "BTCUSDT",
                "bids": [
                    {"price": "16578.50", "size": "0.5"},
                    {"price": "16578.00", "size": "0.3"}
                ],
                "asks": [
                    {"price": "16579.00", "size": "0.4"},
                    {"price": "16579.50", "size": "0.2"}
                ]
            }
        }"#;
        let l2: BybitOrderBookL2 = serde_json::from_str(json).unwrap();
        assert_eq!(l2.data.market, "BTCUSDT");
        assert_eq!(l2.data.bids.len(), 2);
        assert_eq!(l2.data.asks.len(), 2);

        let now = Utc::now();
        let canonical = l2.data.canonicalize(now);
        use crate::books::Level;
        use rust_decimal::Decimal;
        // OrderBook::new sets sequence=0, time_engine=Some(now)
        assert_eq!(canonical.sequence, 0);
        assert_eq!(canonical.time_engine, Some(now));
        assert_eq!(
            canonical.bids().levels()[0],
            Level::new(
                Decimal::from_f64(16578.50).unwrap(),
                Decimal::from_f64(0.5).unwrap()
            )
        );
        assert_eq!(
            canonical.asks().levels()[1],
            Level::new(
                Decimal::from_f64(16579.50).unwrap(),
                Decimal::from_f64(0.2).unwrap()
            )
        );
    }
}
