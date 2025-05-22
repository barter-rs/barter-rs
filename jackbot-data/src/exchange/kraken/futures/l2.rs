use super::super::KrakenChannel;
use crate::{
    Identifier, SnapshotFetcher,
    books::{Canonicalizer, Level, OrderBook},
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::kraken::Kraken,
    instrument::InstrumentData,
    subscription::{
        Map, Subscription,
        book::{OrderBookEvent, OrderBooksL2},
    },
    transformer::ExchangeTransformer,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use jackbot_instrument::exchange::ExchangeId;
use jackbot_integration::{
    Transformer, error::SocketError, protocol::websocket::WsMessage, subscription::SubscriptionId,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::future::Future;
use tokio::sync::mpsc::UnboundedSender;

/// Kraken HTTP OrderBook L2 snapshot URL for futures
/// See docs: https://docs.futures.kraken.com/rest/#/Market%20Data/get_orderbook
pub const HTTP_BOOK_L2_SNAPSHOT_URL_KRAKEN_FUTURES: &str =
    "https://futures.kraken.com/derivatives/api/v3/orderbook";

/// [`Kraken`](super::super::Kraken) Futures L2 OrderBook message.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.futures.kraken.com/#websocket-api-public-feeds-book>
/// ```json
/// {
///   "feed": "book",
///   "product_id": "PI_XBTUSD",
///   "side": "sell",
///   "seq": 36728,
///   "price": 8800.0,
///   "qty": 100,
///   "timestamp": 1565513976274
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct KrakenFuturesOrderBookL2 {
    pub feed: String,
    pub product_id: String,
    #[serde(default)]
    pub side: Option<String>,
    pub seq: u64,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub qty: Option<f64>,
    pub timestamp: u64,
    #[serde(default)]
    pub bids: Option<Vec<[f64; 2]>>, // Snapshot bids [price, quantity]
    #[serde(default)]
    pub asks: Option<Vec<[f64; 2]>>, // Snapshot asks [price, quantity]
}

impl Identifier<Option<SubscriptionId>> for KrakenFuturesOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(SubscriptionId::from(format!(
            "{}|{}",
            KrakenChannel::ORDER_BOOK_L2.0,
            self.product_id
        )))
    }
}

impl Canonicalizer for KrakenFuturesOrderBookL2 {
    fn canonicalize(&self, timestamp: DateTime<Utc>) -> OrderBook {
        use rust_decimal::prelude::FromPrimitive;

        // Create the order book state
        let mut bids = Vec::new();
        let mut asks = Vec::new();

        // Check if this is a snapshot
        if let (Some(snapshot_bids), Some(snapshot_asks)) = (&self.bids, &self.asks) {
            // Process snapshot
            for &[price, qty] in snapshot_bids {
                if let (Some(price_dec), Some(qty_dec)) =
                    (Decimal::from_f64(price), Decimal::from_f64(qty))
                {
                    if !qty_dec.is_zero() {
                        bids.push(Level {
                            price: price_dec,
                            amount: qty_dec,
                        });
                    }
                }
            }

            for &[price, qty] in snapshot_asks {
                if let (Some(price_dec), Some(qty_dec)) =
                    (Decimal::from_f64(price), Decimal::from_f64(qty))
                {
                    if !qty_dec.is_zero() {
                        asks.push(Level {
                            price: price_dec,
                            amount: qty_dec,
                        });
                    }
                }
            }
        } else if let (Some(side), Some(price), Some(qty)) = (&self.side, self.price, self.qty) {
            // Process update for a single level
            if let (Some(price_dec), Some(qty_dec)) =
                (Decimal::from_f64(price), Decimal::from_f64(qty))
            {
                if !qty_dec.is_zero() {
                    match side.as_str() {
                        "buy" => {
                            bids.push(Level {
                                price: price_dec,
                                amount: qty_dec,
                            });
                        }
                        "sell" => {
                            asks.push(Level {
                                price: price_dec,
                                amount: qty_dec,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        OrderBook::new(self.seq, Some(timestamp), bids, asks)
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, KrakenFuturesOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, orderbook): (ExchangeId, InstrumentKey, KrakenFuturesOrderBookL2),
    ) -> Self {
        // Convert timestamp to DateTime
        let time_exchange = chrono::DateTime::from_timestamp_millis(orderbook.timestamp as i64)
            .unwrap_or_else(|| Utc::now());

        // Canonicalize the orderbook
        let order_book = orderbook.canonicalize(time_exchange);

        // Determine if this is a snapshot or update based on the feed field
        let is_snapshot = orderbook.feed.contains("snapshot");
        let order_book_event = if is_snapshot {
            OrderBookEvent::Snapshot(order_book)
        } else {
            OrderBookEvent::Update(order_book)
        };

        vec![Ok(MarketEvent {
            time_exchange,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: order_book_event,
        })]
        .into_iter()
        .collect()
    }
}

#[derive(Debug)]
pub struct KrakenFuturesOrderBooksL2SnapshotFetcher;

impl SnapshotFetcher<Kraken, OrderBooksL2> for KrakenFuturesOrderBooksL2SnapshotFetcher {
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<Kraken, Instrument, OrderBooksL2>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>>
    + Send
    where
        Instrument: InstrumentData,
        Subscription<Kraken, Instrument, OrderBooksL2>:
            Identifier<super::super::market::KrakenMarket>,
    {
        let l2_snapshot_futures = subscriptions.iter().map(|subscription| {
            // Get market symbol from subscription
            let market = subscription.id();

            // Construct initial OrderBook snapshot GET url
            let snapshot_url = format!(
                "{}?symbol={}",
                HTTP_BOOK_L2_SNAPSHOT_URL_KRAKEN_FUTURES,
                market.as_ref(),
            );

            async move {
                // Fetch initial OrderBook snapshot via HTTP
                let response = reqwest::get(&snapshot_url)
                    .await
                    .map_err(SocketError::Http)?
                    .json::<serde_json::Value>()
                    .await
                    .map_err(SocketError::Http)?;

                // Parse the Kraken response format
                let orderbook = response["orderBook"].clone();

                // Create the KrakenFuturesOrderBookL2 from the response
                let timestamp = Utc::now().timestamp_millis() as u64;
                let seq = 0; // Kraken doesn't provide sequence in snapshot

                let mut bids = Vec::new();
                let mut asks = Vec::new();

                if let Some(bids_array) = orderbook["bids"].as_array() {
                    for bid in bids_array {
                        if let (Some(price), Some(qty)) = (bid[0].as_f64(), bid[1].as_f64()) {
                            bids.push([price, qty]);
                        }
                    }
                }

                if let Some(asks_array) = orderbook["asks"].as_array() {
                    for ask in asks_array {
                        if let (Some(price), Some(qty)) = (ask[0].as_f64(), ask[1].as_f64()) {
                            asks.push([price, qty]);
                        }
                    }
                }

                let kraken_book = KrakenFuturesOrderBookL2 {
                    feed: "book_snapshot".to_string(),
                    product_id: market.as_ref().to_string(),
                    side: None,
                    seq,
                    price: None,
                    qty: None,
                    timestamp,
                    bids: Some(bids),
                    asks: Some(asks),
                };

                // Convert to MarketEvent using the From implementation
                let events = MarketIter::from((
                    ExchangeId::Kraken,
                    subscription.instrument.key().clone(),
                    kraken_book,
                ));

                // Return the first event (should be a snapshot)
                events
                    .0
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| {
                        Err(DataError::InitialSnapshotInvalid(
                            "Missing first event".to_string(),
                        ))
                    })
                    .map_err(|e| SocketError::Exchange(format!("DataError: {}", e)))
            }
        });

        try_join_all(l2_snapshot_futures)
    }
}

#[derive(Debug)]
pub struct KrakenFuturesOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<InstrumentKey>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<Kraken, InstrumentKey, OrderBooksL2>
    for KrakenFuturesOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync,
{
    async fn init(
        instrument_map: Map<InstrumentKey>,
        _initial_snapshots: &[MarketEvent<InstrumentKey, OrderBookEvent>],
        _: UnboundedSender<WsMessage>,
    ) -> Result<Self, DataError> {
        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for KrakenFuturesOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = KrakenFuturesOrderBookL2;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Get subscription ID from the input
        let subscription_id = input.id().unwrap_or_else(|| {
            SubscriptionId::from(format!("{}|unknown", KrakenChannel::ORDER_BOOK_L2.0))
        });

        // Find Instrument associated with Input
        let instrument_key = match self.instrument_map.find(&subscription_id) {
            Ok(key) => key.clone(),
            Err(unidentifiable) => return vec![Err(DataError::from(unidentifiable))],
        };

        // Transform using the From implementation
        MarketIter::from((ExchangeId::Kraken, instrument_key, input)).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_kraken_futures_orderbook_l2_snapshot() {
        let json = r#"{
            "feed": "book_snapshot",
            "product_id": "PI_XBTUSD",
            "timestamp": 1565513976274,
            "seq": 36728,
            "bids": [
                [8800.0, 100.0],
                [8795.0, 50.0]
            ],
            "asks": [
                [8810.0, 75.0],
                [8820.0, 25.0]
            ]
        }"#;

        let orderbook: KrakenFuturesOrderBookL2 = serde_json::from_str(json).unwrap();

        assert_eq!(orderbook.feed, "book_snapshot");
        assert_eq!(orderbook.product_id, "PI_XBTUSD");
        assert_eq!(orderbook.timestamp, 1565513976274);
        assert_eq!(orderbook.seq, 36728);
        assert_eq!(orderbook.bids.as_ref().unwrap()[0], [8800.0, 100.0]);
        assert_eq!(orderbook.asks.as_ref().unwrap()[1], [8820.0, 25.0]);

        let market_iter =
            MarketIter::from((ExchangeId::Kraken, "PI_XBTUSD".to_string(), orderbook));

        let events = market_iter.0;
        assert_eq!(events.len(), 1);

        let event = events[0].as_ref().unwrap();
        if let OrderBookEvent::Snapshot(book) = &event.kind {
            assert_eq!(book.sequence, 36728);
            assert_eq!(
                book.bids().levels()[0],
                Level::new(dec!(8800.0), dec!(100.0))
            );
            assert_eq!(
                book.asks().levels()[1],
                Level::new(dec!(8820.0), dec!(25.0))
            );
        } else {
            panic!("Expected OrderBookEvent::Snapshot");
        }
    }

    #[test]
    fn test_kraken_futures_orderbook_l2_update() {
        let json = r#"{
            "feed": "book",
            "product_id": "PI_XBTUSD",
            "side": "sell",
            "seq": 36729,
            "price": 8815.0,
            "qty": 30.0,
            "timestamp": 1565513986274
        }"#;

        let orderbook: KrakenFuturesOrderBookL2 = serde_json::from_str(json).unwrap();

        assert_eq!(orderbook.feed, "book");
        assert_eq!(orderbook.product_id, "PI_XBTUSD");
        assert_eq!(orderbook.timestamp, 1565513986274);
        assert_eq!(orderbook.seq, 36729);
        assert_eq!(orderbook.side, Some("sell".to_string()));
        assert_eq!(orderbook.price, Some(8815.0));
        assert_eq!(orderbook.qty, Some(30.0));

        let market_iter =
            MarketIter::from((ExchangeId::Kraken, "PI_XBTUSD".to_string(), orderbook));

        let events = market_iter.0;
        assert_eq!(events.len(), 1);

        let event = events[0].as_ref().unwrap();
        if let OrderBookEvent::Update(book) = &event.kind {
            assert_eq!(book.sequence, 36729);
            assert_eq!(
                book.asks().levels()[0],
                Level::new(dec!(8815.0), dec!(30.0))
            );
        } else {
            panic!("Expected OrderBookEvent::Update");
        }
    }
}
