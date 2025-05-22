use super::super::CoinbaseChannel;
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
use std::str::FromStr;

/// [`Coinbase`](super::super::Coinbase) L2 OrderBook message.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#full>
/// ```json
/// {
///   "type": "snapshot",
///   "product_id": "BTC-USD",
///   "asks": [
///     ["10106.80000000", "1.20000000"]
///   ],
///   "bids": [
///     ["10101.10000000", "0.45054140"]
///   ]
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL2Snapshot {
    pub product_id: String,
    pub asks: Vec<[String; 2]>,
    pub bids: Vec<[String; 2]>,
    #[serde(skip)]
    pub time: DateTime<Utc>,
}

/// [`Coinbase`](super::super::Coinbase) L2 OrderBook update message.
///
/// ### Raw Payload Examples
/// ```json
/// {
///   "type": "l2update",
///   "product_id": "BTC-USD",
///   "time": "2019-08-14T20:42:27.265Z",
///   "changes": [
///     [
///       "buy",
///       "10101.80000000",
///       "0.162567"
///     ]
///   ]
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL2Update {
    pub product_id: String,
    pub time: Option<DateTime<Utc>>,
    pub changes: Vec<CoinbaseOrderBookL2Change>,
}

/// [`Coinbase`](super::super::Coinbase) L2 OrderBook change.
///
/// ### Raw Payload Examples
/// ```json
/// [
///   "buy",
///   "10101.80000000",
///   "0.162567"
/// ]
/// ```
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#full>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct CoinbaseOrderBookL2Change(
    pub String, // Side: "buy" or "sell"
    pub String, // Price
    pub String, // Size (quantity)
);

impl Identifier<Option<SubscriptionId>> for CoinbaseOrderBookL2Snapshot {
    fn id(&self) -> Option<SubscriptionId> {
        Some(SubscriptionId::from(format!(
            "{}|{}",
            CoinbaseChannel::BOOK_L2.0,
            self.product_id
        )))
    }
}

impl Identifier<Option<SubscriptionId>> for CoinbaseOrderBookL2Update {
    fn id(&self) -> Option<SubscriptionId> {
        Some(SubscriptionId::from(format!(
            "{}|{}",
            CoinbaseChannel::BOOK_L2.0,
            self.product_id
        )))
    }
}

// Add Canonicalizer implementation for CoinbaseOrderBookL2Snapshot
impl Canonicalizer for CoinbaseOrderBookL2Snapshot {
    fn canonicalize(&self, timestamp: DateTime<Utc>) -> OrderBook {
        let time = if self.time != DateTime::<Utc>::default() {
            self.time
        } else {
            timestamp
        };

        OrderBook::new(
            0, // No sequence number provided
            Some(time),
            self.bids.iter().filter_map(|[price, amount]| {
                let price = Decimal::from_str_exact(price).ok()?;
                let amount = Decimal::from_str_exact(amount).ok()?;
                Some(Level::new(price, amount))
            }),
            self.asks.iter().filter_map(|[price, amount]| {
                let price = Decimal::from_str_exact(price).ok()?;
                let amount = Decimal::from_str_exact(amount).ok()?;
                Some(Level::new(price, amount))
            }),
        )
    }
}

// Add Canonicalizer implementation for CoinbaseOrderBookL2Update
impl Canonicalizer for CoinbaseOrderBookL2Update {
    fn canonicalize(&self, timestamp: DateTime<Utc>) -> OrderBook {
        let time = self.time.unwrap_or(timestamp);

        // Process updates
        let mut bids = Vec::new();
        let mut asks = Vec::new();

        for change in &self.changes {
            if let (Ok(price), Ok(amount)) = (
                Decimal::from_str_exact(&change.1),
                Decimal::from_str_exact(&change.2),
            ) {
                let level = Level::new(price, amount);
                match change.0.as_str() {
                    "buy" => bids.push(level),
                    "sell" => asks.push(level),
                    _ => continue,
                }
            }
        }

        OrderBook::new(
            0, // No sequence number provided
            Some(time),
            bids.into_iter(),
            asks.into_iter(),
        )
    }
}

// Update the From implementation to use Canonicalizer
impl<InstrumentKey> From<(ExchangeId, InstrumentKey, CoinbaseOrderBookL2Snapshot)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, snapshot): (
            ExchangeId,
            InstrumentKey,
            CoinbaseOrderBookL2Snapshot,
        ),
    ) -> Self {
        // Use the Canonicalizer to create the OrderBook
        let order_book = snapshot.canonicalize(Utc::now());

        Self(vec![Ok(MarketEvent {
            time_exchange: snapshot.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: OrderBookEvent::Snapshot(order_book),
        })])
    }
}

// Update the From implementation to use Canonicalizer
impl<InstrumentKey> From<(ExchangeId, InstrumentKey, CoinbaseOrderBookL2Update)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, update): (ExchangeId, InstrumentKey, CoinbaseOrderBookL2Update),
    ) -> Self {
        let time = update.time.unwrap_or_else(Utc::now);

        // Use the Canonicalizer to create the OrderBook
        let delta_book = update.canonicalize(time);

        Self(vec![Ok(MarketEvent {
            time_exchange: time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: OrderBookEvent::Update(delta_book),
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_coinbase_orderbook_l2_snapshot() {
        let input = r#"
        {
            "type": "snapshot",
            "product_id": "BTC-USD",
            "asks": [
                ["10106.80000000", "1.20000000"]
            ],
            "bids": [
                ["10101.10000000", "0.45054140"]
            ]
        }
        "#;

        let mut snapshot = serde_json::from_str::<CoinbaseOrderBookL2Snapshot>(input).unwrap();
        snapshot.time = Utc::now(); // Set time since it's skipped in deserialization

        // Create MarketIter from the parsed snapshot
        let market_iter = MarketIter::<String, OrderBookEvent>::from((
            ExchangeId::Coinbase,
            "BTC-USD".to_string(),
            snapshot,
        ));

        // Get the first event
        let event = market_iter.0.first().unwrap().as_ref().unwrap();

        if let OrderBookEvent::Snapshot(book) = &event.kind {
            // Use the proper accessor method to get levels from the book sides
            let bids = book.bids().levels();
            let asks = book.asks().levels();

            assert_eq!(bids.len(), 1);
            assert_eq!(asks.len(), 1);

            assert_eq!(bids[0].price, dec!(10101.10000000));
            assert_eq!(bids[0].amount, dec!(0.45054140));
            assert_eq!(asks[0].price, dec!(10106.80000000));
            assert_eq!(asks[0].amount, dec!(1.20000000));
        } else {
            panic!("Expected OrderBookEvent::Snapshot");
        }
    }

    #[test]
    fn test_coinbase_orderbook_l2_update() {
        let input = r#"
        {
            "type": "l2update",
            "product_id": "BTC-USD",
            "time": "2019-08-14T20:42:27.265Z",
            "changes": [
                [
                    "buy",
                    "10101.80000000",
                    "0.162567"
                ],
                [
                    "sell",
                    "10106.00000000",
                    "0.39748"
                ]
            ]
        }
        "#;

        let update = serde_json::from_str::<CoinbaseOrderBookL2Update>(input).unwrap();

        // Create MarketIter from the parsed update
        let market_iter = MarketIter::<String, OrderBookEvent>::from((
            ExchangeId::Coinbase,
            "BTC-USD".to_string(),
            update,
        ));

        // Get the first event
        let event = market_iter.0.first().unwrap().as_ref().unwrap();

        if let OrderBookEvent::Update(book) = &event.kind {
            // Use the proper accessor method to get levels from the book sides
            let bids = book.bids().levels();
            let asks = book.asks().levels();

            assert_eq!(bids.len(), 1);
            assert_eq!(asks.len(), 1);

            assert_eq!(bids[0].price, dec!(10101.80000000));
            assert_eq!(bids[0].amount, dec!(0.162567));
            assert_eq!(asks[0].price, dec!(10106.00000000));
            assert_eq!(asks[0].amount, dec!(0.39748));
        } else {
            panic!("Expected OrderBookEvent::Update");
        }
    }
}
