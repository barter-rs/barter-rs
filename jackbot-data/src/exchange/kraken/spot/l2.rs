use super::super::KrakenChannel;
use crate::{
    Identifier,
    books::{Canonicalizer, Level, OrderBook},
    event::{MarketEvent, MarketIter},
    subscription::book::OrderBookEvent,
};
use chrono::{DateTime, Utc};
use jackbot_instrument::exchange::ExchangeId;
use jackbot_integration::subscription::SubscriptionId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// [`Kraken`](super::super::Kraken) L2 OrderBook message.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.kraken.com/websockets/#message-book>
/// ```json
/// [
///   0,
///   {
///     "as": [
///       ["5541.30000", "2.50700000", "1534614248.123678"],
///       ["5542.50000", "0.40100000", "1534614248.456789"]
///     ],
///     "bs": [
///       ["5541.20000", "1.52900000", "1534614248.765432"],
///       ["5539.90000", "0.30000000", "1534614243.345678"]
///     ]
///   },
///   "book-25",
///   "XBT/USD"
/// ]
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct KrakenOrderBookL2(
    pub i32, // Channel ID (ignore)
    pub KrakenOrderBookL2Data,
    pub String, // Channel name (e.g., "book-25")
    pub String, // Market (e.g., "XBT/USD")
);

impl Canonicalizer for KrakenOrderBookL2 {
    fn canonicalize(&self, _timestamp: DateTime<Utc>) -> OrderBook {
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        let mut latest_time = Utc::now();

        if let Some(snapshot_asks) = &self.1.as_ {
            for ask in snapshot_asks {
                if let (Ok(price), Ok(amount), Ok(epoch_seconds)) = (
                    ask[0].parse::<Decimal>(),
                    ask[1].parse::<Decimal>(),
                    ask[2].parse::<f64>(),
                ) {
                    let seconds = epoch_seconds.trunc() as i64;
                    let nanos = ((epoch_seconds.fract() * 1_000_000_000.0).round() as u32).min(999_999_999);
                    if let Some(timestamp) = DateTime::from_timestamp(seconds, nanos) {
                        if timestamp > latest_time {
                            latest_time = timestamp;
                        }

                        if !amount.is_zero() {
                            asks.push(Level { price, amount });
                        }
                    }
                }
            }
        }

        if let Some(snapshot_bids) = &self.1.bs {
            for bid in snapshot_bids {
                if let (Ok(price), Ok(amount), Ok(epoch_seconds)) = (
                    bid[0].parse::<Decimal>(),
                    bid[1].parse::<Decimal>(),
                    bid[2].parse::<f64>(),
                ) {
                    let seconds = epoch_seconds.trunc() as i64;
                    let nanos = ((epoch_seconds.fract() * 1_000_000_000.0).round() as u32).min(999_999_999);
                    if let Some(timestamp) = DateTime::from_timestamp(seconds, nanos) {
                        if timestamp > latest_time {
                            latest_time = timestamp;
                        }

                        if !amount.is_zero() {
                            bids.push(Level { price, amount });
                        }
                    }
                }
            }
        }

        if let Some(ask_updates) = &self.1.a {
            for ask in ask_updates {
                if let (Ok(price), Ok(amount), Ok(epoch_seconds)) = (
                    ask[0].parse::<Decimal>(),
                    ask[1].parse::<Decimal>(),
                    ask[2].parse::<f64>(),
                ) {
                    let seconds = epoch_seconds.trunc() as i64;
                    let nanos = ((epoch_seconds.fract() * 1_000_000_000.0).round() as u32).min(999_999_999);
                    if let Some(timestamp) = DateTime::from_timestamp(seconds, nanos) {
                        if timestamp > latest_time {
                            latest_time = timestamp;
                        }

                        if !amount.is_zero() {
                            asks.push(Level { price, amount });
                        }
                    }
                }
            }
        }

        if let Some(bid_updates) = &self.1.b {
            for bid in bid_updates {
                if let (Ok(price), Ok(amount), Ok(epoch_seconds)) = (
                    bid[0].parse::<Decimal>(),
                    bid[1].parse::<Decimal>(),
                    bid[2].parse::<f64>(),
                ) {
                    let seconds = epoch_seconds.trunc() as i64;
                    let nanos = ((epoch_seconds.fract() * 1_000_000_000.0).round() as u32).min(999_999_999);
                    if let Some(timestamp) = DateTime::from_timestamp(seconds, nanos) {
                        if timestamp > latest_time {
                            latest_time = timestamp;
                        }

                        if !amount.is_zero() {
                            bids.push(Level { price, amount });
                        }
                    }
                }
            }
        }

        OrderBook::new(0, Some(latest_time), bids, asks)
    }
}

/// [`Kraken`](super::super::Kraken) L2 OrderBook data structure.
///
/// ### Raw Payload Examples
/// ```json
/// {
///   "as": [
///     ["5541.30000", "2.50700000", "1534614248.123678"],
///     ["5542.50000", "0.40100000", "1534614248.456789"]
///   ],
///   "bs": [
///     ["5541.20000", "1.52900000", "1534614248.765432"],
///     ["5539.90000", "0.30000000", "1534614243.345678"]
///   ]
/// }
/// ```
///
/// See docs: <https://docs.kraken.com/websockets/#message-book>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct KrakenOrderBookL2Data {
    #[serde(default, alias = "as")]
    pub as_: Option<Vec<[String; 3]>>, // Asks: [price, volume, timestamp]
    #[serde(default, alias = "bs")]
    pub bs: Option<Vec<[String; 3]>>, // Bids: [price, volume, timestamp]
    #[serde(default)]
    pub a: Option<Vec<[String; 3]>>, // Ask updates
    #[serde(default)]
    pub b: Option<Vec<[String; 3]>>, // Bid updates
}

impl Identifier<Option<SubscriptionId>> for KrakenOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(SubscriptionId::from(format!(
            "{}|{}",
            KrakenChannel::ORDER_BOOK_L2.0,
            self.3 // Market name like "XBT/USD"
        )))
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, KrakenOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, orderbook): (ExchangeId, InstrumentKey, KrakenOrderBookL2),
    ) -> Self {
        let orderbook_obj = orderbook.canonicalize(Utc::now());
        let time_exchange = orderbook_obj.time_engine.unwrap_or_else(Utc::now);
        let event = if orderbook.1.as_.is_some() || orderbook.1.bs.is_some() {
            OrderBookEvent::Snapshot(orderbook_obj)
        } else {
            OrderBookEvent::Update(orderbook_obj)
        };

        Self(vec![Ok(MarketEvent {
            time_exchange,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: event,
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_kraken_orderbook_l2_snapshot() {
            let input = r#"
            [
              0,
              {
                "as": [
                  ["5541.30000", "2.50700000", "1534614248.123678"],
                  ["5542.50000", "0.40100000", "1534614248.456789"]
                ],
                "bs": [
                  ["5541.20000", "1.52900000", "1534614248.765432"],
                  ["5539.90000", "0.30000000", "1534614243.345678"]
                ]
              },
              "book-25",
              "XBT/USD"
            ]
            "#;

            let orderbook = serde_json::from_str::<KrakenOrderBookL2>(input).unwrap();

            assert_eq!(orderbook.0, 0);
            assert_eq!(orderbook.2, "book-25");
            assert_eq!(orderbook.3, "XBT/USD");

            let data = &orderbook.1;
            assert!(data.as_.is_some());
            assert!(data.bs.is_some());
            assert!(data.a.is_none());
            assert!(data.b.is_none());

            let asks = data.as_.as_ref().unwrap();
            assert_eq!(asks.len(), 2);
            assert_eq!(asks[0][0], "5541.30000");
            assert_eq!(asks[0][1], "2.50700000");
            assert_eq!(asks[0][2], "1534614248.123678");

            let bids = data.bs.as_ref().unwrap();
            assert_eq!(bids.len(), 2);
            assert_eq!(bids[0][0], "5541.20000");
            assert_eq!(bids[0][1], "1.52900000");
            assert_eq!(bids[0][2], "1534614248.765432");
        }

        #[test]
        fn test_kraken_orderbook_l2_update() {
            let input = r#"
            [
              0,
              {
                "a": [
                  ["5541.30000", "2.50700000", "1534614248.123678"],
                  ["5542.50000", "0.00000000", "1534614248.456789"]
                ],
                "b": [
                  ["5541.20000", "1.52900000", "1534614248.765432"]
                ]
              },
              "book-25",
              "XBT/USD"
            ]
            "#;

            let orderbook = serde_json::from_str::<KrakenOrderBookL2>(input).unwrap();

            assert_eq!(orderbook.0, 0);
            assert_eq!(orderbook.2, "book-25");
            assert_eq!(orderbook.3, "XBT/USD");

            let data = &orderbook.1;
            assert!(data.as_.is_none());
            assert!(data.bs.is_none());
            assert!(data.a.is_some());
            assert!(data.b.is_some());

            let ask_updates = data.a.as_ref().unwrap();
            assert_eq!(ask_updates.len(), 2);
            assert_eq!(ask_updates[0][0], "5541.30000");
            assert_eq!(ask_updates[0][1], "2.50700000");
            assert_eq!(ask_updates[0][2], "1534614248.123678");
            assert_eq!(ask_updates[1][0], "5542.50000");
            assert_eq!(ask_updates[1][1], "0.00000000"); // This should remove the price level

            let bid_updates = data.b.as_ref().unwrap();
            assert_eq!(bid_updates.len(), 1);
            assert_eq!(bid_updates[0][0], "5541.20000");
            assert_eq!(bid_updates[0][1], "1.52900000");
            assert_eq!(bid_updates[0][2], "1534614248.765432");
        }
    }
}
