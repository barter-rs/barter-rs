use crate::{
    Identifier, SnapshotFetcher,
    books::{
        Level, OrderBook,
        l2_sequencer::{HasUpdateIds, L2Sequencer},
    },
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::{
        Connector,
        okx::{Okx, channel::OkxChannel, market::OkxMarket},
        subscription::ExchangeSub,
    },
    instrument::InstrumentData,
    subscription::{
        Map, Subscription,
        book::{OrderBookEvent, OrderBooksL2},
    },
    transformer::ExchangeTransformer,
};
use async_trait::async_trait;
use chrono::Utc;
use futures_util::{FutureExt, future::try_join_all};
use jackbot_instrument::exchange::ExchangeId;
use jackbot_integration::{
    Transformer, error::SocketError, protocol::websocket::WsMessage, subscription::SubscriptionId,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::future::Future;
use tokio::sync::mpsc::UnboundedSender;

/// OKX HTTP OrderBook L2 snapshot URL
/// See docs: https://www.okx.com/docs-v5/en/#order-book-trading-market-data-get-order-book
pub const HTTP_BOOK_L2_SNAPSHOT_URL_OKX_SPOT: &str = "https://www.okx.com/api/v5/market/books";

/// OKX real-time OrderBook Level2 message.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OkxOrderBookL2 {
    #[serde(alias = "instId", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(default = "Utc::now")]
    pub time: chrono::DateTime<Utc>,
    #[serde(alias = "bids")]
    pub bids: Vec<(Decimal, Decimal)>,
    #[serde(alias = "asks")]
    pub asks: Vec<(Decimal, Decimal)>,
    #[serde(default)]
    pub action: Option<String>, // "snapshot" or "update"
}

impl Identifier<Option<SubscriptionId>> for OkxOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, OkxOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from((exchange_id, instrument, book): (ExchangeId, InstrumentKey, OkxOrderBookL2)) -> Self {
        let bids: Vec<Level> = book.bids.iter().map(|(p, a)| Level::new(*p, *a)).collect();
        let asks: Vec<Level> = book.asks.iter().map(|(p, a)| Level::new(*p, *a)).collect();

        let order_book = OrderBook::new(0, Some(book.time), bids, asks);

        // Determine if this is a snapshot or update based on the action field
        // OKX uses "snapshot" for initial snapshots and "update" for incremental updates
        let event = match book.action.as_deref() {
            Some("snapshot") => OrderBookEvent::Snapshot(order_book),
            _ => OrderBookEvent::Update(order_book),
        };

        Self(vec![Ok(MarketEvent {
            time_exchange: book.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: event,
        })])
    }
}

/// Deserialize an OkxOrderBookL2 "instId" as the associated SubscriptionId.
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((OkxChannel::ORDER_BOOK_L2, market)).id())
}

/// Sequencer implementation for OKX Spot order book.
#[derive(Debug, Clone)]
pub struct OkxSpotOrderBookL2Sequencer {
    pub last_update_id: u64,
    pub updates_processed: u64,
}

impl L2Sequencer<OkxOrderBookL2> for OkxSpotOrderBookL2Sequencer {
    fn new(last_update_id: u64) -> Self {
        Self {
            last_update_id,
            updates_processed: 0,
        }
    }

    fn validate_sequence(
        &mut self,
        update: OkxOrderBookL2,
    ) -> Result<Option<OkxOrderBookL2>, DataError> {
        // OKX doesn't provide sequence numbers for orderbook updates
        // We rely on the action field to determine if it's a snapshot or an update
        self.updates_processed += 1;
        Ok(Some(update))
    }

    fn is_first_update(&self) -> bool {
        self.updates_processed == 0
    }
}

#[derive(Debug)]
pub struct OkxSpotOrderBooksL2SnapshotFetcher;

impl SnapshotFetcher<Okx, OrderBooksL2> for OkxSpotOrderBooksL2SnapshotFetcher {
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<Okx, Instrument, OrderBooksL2>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>>
    + Send
    where
        Instrument: InstrumentData,
        Subscription<Okx, Instrument, OrderBooksL2>: Identifier<OkxMarket>,
    {
        let l2_snapshot_futures = subscriptions.iter().map(|subscription| {
            // Construct initial OrderBook snapshot GET url
            let market = subscription.id();
            let snapshot_url =
                format!("{}?instId={}", HTTP_BOOK_L2_SNAPSHOT_URL_OKX_SPOT, market.0,);

            async move {
                // Fetch initial OrderBook snapshot via HTTP
                let response = reqwest::get(snapshot_url)
                    .await
                    .map_err(SocketError::Http)?
                    .json::<serde_json::Value>()
                    .await
                    .map_err(SocketError::Http)?;

                // Extract the data from the OKX response format
                if let Some(data) = response["data"].as_array() {
                    if let Some(first_item) = data.first() {
                        // Build the OkxOrderBookL2 manually
                        let mut book = OkxOrderBookL2 {
                            subscription_id: SubscriptionId::from(market.0.to_string()),
                            time: chrono::Utc::now(), // Set current time
                            bids: Vec::new(),
                            asks: Vec::new(),
                            action: Some("snapshot".to_string()),
                        };

                        // Parse bids
                        if let Some(bids_array) = first_item["bids"].as_array() {
                            for bid in bids_array {
                                if let (Some(price_str), Some(size_str)) =
                                    (bid[0].as_str(), bid[1].as_str())
                                {
                                    if let (Ok(price), Ok(size)) =
                                        (price_str.parse::<Decimal>(), size_str.parse::<Decimal>())
                                    {
                                        book.bids.push((price, size));
                                    }
                                }
                            }
                        }

                        // Parse asks
                        if let Some(asks_array) = first_item["asks"].as_array() {
                            for ask in asks_array {
                                if let (Some(price_str), Some(size_str)) =
                                    (ask[0].as_str(), ask[1].as_str())
                                {
                                    if let (Ok(price), Ok(size)) =
                                        (price_str.parse::<Decimal>(), size_str.parse::<Decimal>())
                                    {
                                        book.asks.push((price, size));
                                    }
                                }
                            }
                        }

                        // Convert to MarketEvent
                        let market_iter = MarketIter::from((
                            ExchangeId::Okx,
                            subscription.instrument.key().clone(),
                            book,
                        ));

                        // If we have at least one event, return it
                        if !market_iter.0.is_empty() {
                            let events = market_iter
                                .0
                                .into_iter()
                                .map(|e| e.map_err(|e| SocketError::Exchange(format!("{:?}", e))))
                                .collect::<Result<Vec<_>, _>>()?;
                            return Ok(events);
                        }
                    }
                }

                // If we reach here, we couldn't parse the response
                Err(SocketError::Exchange(String::from(
                    "Failed to parse OKX orderbook response",
                )))
            }
        });

        try_join_all(l2_snapshot_futures).map(|results| {
            results.map(|nested_events| nested_events.into_iter().flatten().collect())
        })
    }
}

#[derive(Debug)]
pub struct OkxSpotOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<(InstrumentKey, OkxSpotOrderBookL2Sequencer)>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<Okx, InstrumentKey, OrderBooksL2>
    for OkxSpotOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync,
{
    async fn init(
        instrument_map: Map<InstrumentKey>,
        initial_snapshots: &[MarketEvent<InstrumentKey, OrderBookEvent>],
        _: UnboundedSender<WsMessage>,
    ) -> Result<Self, DataError> {
        let instrument_map = instrument_map
            .0
            .into_iter()
            .map(|(sub_id, instrument_key)| {
                let snapshot = initial_snapshots
                    .iter()
                    .find(|snapshot| snapshot.instrument == instrument_key)
                    .ok_or_else(|| DataError::InitialSnapshotMissing(sub_id.clone()))?;

                let OrderBookEvent::Snapshot(snapshot) = &snapshot.kind else {
                    return Err(DataError::InitialSnapshotInvalid(String::from(
                        "expected OrderBookEvent::Snapshot but found OrderBookEvent::Update",
                    )));
                };

                let sequencer = <OkxSpotOrderBookL2Sequencer as L2Sequencer<OkxOrderBookL2>>::new(
                    snapshot.sequence,
                );

                Ok((sub_id, (instrument_key, sequencer)))
            })
            .collect::<Result<Map<_>, _>>()?;

        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for OkxSpotOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = OkxOrderBookL2;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Find Instrument associated with Input
        let (instrument_key, sequencer) = match self.instrument_map.find_mut(&input.subscription_id)
        {
            Ok(tuple) => tuple,
            Err(unidentifiable) => return vec![Err(DataError::from(unidentifiable))],
        };

        // Apply sequencing logic
        let input = match sequencer.validate_sequence(input) {
            Ok(Some(input)) => input,
            Ok(None) => return Vec::new(), // Drop the update
            Err(e) => return vec![Err(e)],
        };

        // Transform the message
        let mut events = Vec::new();
        for event in MarketIter::from((ExchangeId::Okx, instrument_key.clone(), input)).0 {
            events.push(event);
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{Asks, Bids};
    use jackbot_instrument::exchange::ExchangeId;
    use rust_decimal_macros::dec;

    #[test]
    fn test_okx_order_book_l2_update() {
        let input = r#"{"instId":"BTC-USDT","bids":[["30000.0","1.0"]],"asks":[["30010.0","2.0"]],"action":"update"}"#;
        let book: OkxOrderBookL2 = serde_json::from_str(input).unwrap();
        assert_eq!(book.bids[0], (dec!(30000.0), dec!(1.0)));
        assert_eq!(book.asks[0], (dec!(30010.0), dec!(2.0)));
        assert_eq!(book.action, Some("update".to_string()));
    }

    #[test]
    fn test_okx_order_book_l2_snapshot() {
        let input = r#"{"instId":"BTC-USDT","bids":[["30000.0","1.0"],["29990.0","2.0"]],"asks":[["30010.0","2.0"],["30020.0","3.0"]],"action":"snapshot"}"#;
        let book: OkxOrderBookL2 = serde_json::from_str(input).unwrap();
        assert_eq!(book.bids[0], (dec!(30000.0), dec!(1.0)));
        assert_eq!(book.bids[1], (dec!(29990.0), dec!(2.0)));
        assert_eq!(book.asks[0], (dec!(30010.0), dec!(2.0)));
        assert_eq!(book.asks[1], (dec!(30020.0), dec!(3.0)));
        assert_eq!(book.action, Some("snapshot".to_string()));
    }

    #[test]
    fn test_okx_order_book_transform_snapshot() {
        let input = r#"{"instId":"BTC-USDT","bids":[["30000.0","1.0"]],"asks":[["30010.0","2.0"]],"action":"snapshot"}"#;
        let book: OkxOrderBookL2 = serde_json::from_str(input).unwrap();

        let mut sequencer = OkxSpotOrderBookL2Sequencer::new(0);
        let mut transformer = OkxSpotOrderBooksL2Transformer {
            instrument_map: Map(vec![(
                SubscriptionId::from("BTC-USDT".to_string()),
                ("BTC-USDT".to_string(), sequencer),
            )]
            .into_iter()
            .collect()),
        };

        let result = transformer.transform(book);
        if let Some(Ok(event)) = result.first() {
            if let OrderBookEvent::Snapshot(ob) = &event.kind {
                // Access the book sides
                let bids_levels = ob.bids().levels();
                let asks_levels = ob.asks().levels();

                assert_eq!(bids_levels.len(), 1);
                assert_eq!(asks_levels.len(), 1);
                assert_eq!(bids_levels[0].price, dec!(30000.0));
                assert_eq!(bids_levels[0].amount, dec!(1.0));
                assert_eq!(asks_levels[0].price, dec!(30010.0));
                assert_eq!(asks_levels[0].amount, dec!(2.0));
            } else {
                panic!("Expected OrderBookEvent::Snapshot");
            }
        } else {
            panic!("Expected at least one event");
        }
    }

    #[test]
    fn test_okx_order_book_transform_update() {
        let input = r#"{"instId":"BTC-USDT","bids":[["30000.0","1.0"]],"asks":[["30010.0","2.0"]],"action":"update"}"#;
        let book: OkxOrderBookL2 = serde_json::from_str(input).unwrap();

        let mut sequencer = OkxSpotOrderBookL2Sequencer::new(0);
        let mut transformer = OkxSpotOrderBooksL2Transformer {
            instrument_map: Map(vec![(
                SubscriptionId::from("BTC-USDT".to_string()),
                ("BTC-USDT".to_string(), sequencer),
            )]
            .into_iter()
            .collect()),
        };

        let result = transformer.transform(book);
        if let Some(Ok(event)) = result.first() {
            if let OrderBookEvent::Update(ob) = &event.kind {
                // Access the book sides
                let bids_levels = ob.bids().levels();
                let asks_levels = ob.asks().levels();

                assert_eq!(bids_levels.len(), 1);
                assert_eq!(asks_levels.len(), 1);
                assert_eq!(bids_levels[0].price, dec!(30000.0));
                assert_eq!(bids_levels[0].amount, dec!(1.0));
                assert_eq!(asks_levels[0].price, dec!(30010.0));
                assert_eq!(asks_levels[0].amount, dec!(2.0));
            } else {
                panic!("Expected OrderBookEvent::Update");
            }
        } else {
            panic!("Expected at least one event");
        }
    }

    #[test]
    fn test_okx_sequencer() {
        let mut sequencer = OkxSpotOrderBookL2Sequencer::new(0);
        assert!(sequencer.is_first_update());

        let input = r#"{"instId":"BTC-USDT","bids":[["30000.0","1.0"]],"asks":[["30010.0","2.0"]],"action":"snapshot"}"#;
        let book: OkxOrderBookL2 = serde_json::from_str(input).unwrap();

        let result = sequencer.validate_sequence(book);
        assert!(result.is_ok());
        assert!(!sequencer.is_first_update());
    }
}
