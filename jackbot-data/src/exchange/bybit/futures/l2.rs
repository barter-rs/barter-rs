use super::super::book::{BybitOrderBookL2Data, BybitOrderBookLevel};
use crate::{
    Identifier, SnapshotFetcher,
    books::Canonicalizer,
    error::DataError,
    event::MarketEvent,
    exchange::bybit::{futures::BybitPerpetualsUsd, market::BybitMarket, message::BybitPayload},
    instrument::InstrumentData,
    subscription::{
        Map, Subscription,
        book::{OrderBookEvent, OrderBooksL2},
    },
    transformer::ExchangeTransformer,
};
use async_trait::async_trait;
use chrono::Utc;
use futures_util::future::try_join_all;
use jackbot_instrument::exchange::ExchangeId;
use jackbot_integration::{Transformer, error::SocketError, protocol::websocket::WsMessage};
use std::future::Future;
use tokio::sync::mpsc::UnboundedSender;

/// Bybit HTTP OrderBook L2 snapshot URL for futures
/// See docs: https://bybit-exchange.github.io/docs/v5/market/orderbook
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BYBIT_PERPETUALS_USD: &str =
    "https://api.bybit.com/v5/market/orderbook";

#[derive(Debug)]
pub struct BybitPerpetualsUsdOrderBooksL2SnapshotFetcher;

impl SnapshotFetcher<BybitPerpetualsUsd, OrderBooksL2>
    for BybitPerpetualsUsdOrderBooksL2SnapshotFetcher
{
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<BybitPerpetualsUsd, Instrument, OrderBooksL2>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>>
    + Send
    where
        Instrument: InstrumentData,
        Subscription<BybitPerpetualsUsd, Instrument, OrderBooksL2>: Identifier<BybitMarket>,
    {
        let l2_snapshot_futures = subscriptions.iter().map(|subscription| {
            // Construct initial OrderBook snapshot GET url
            let market = subscription.id();
            let snapshot_url = format!(
                "{}?category=linear&symbol={}&limit=100",
                HTTP_BOOK_L2_SNAPSHOT_URL_BYBIT_PERPETUALS_USD, market.0,
            );

            async move {
                // Fetch initial OrderBook snapshot via HTTP
                let response = reqwest::get(snapshot_url)
                    .await
                    .map_err(SocketError::Http)?
                    .json::<serde_json::Value>()
                    .await
                    .map_err(SocketError::Http)?;

                // Parse the Bybit response format
                let result = response["result"].clone();
                let book_data = BybitOrderBookL2Data {
                    market: result["s"].as_str().unwrap_or_default().to_string(),
                    bids: parse_bybit_levels(&result["b"]),
                    asks: parse_bybit_levels(&result["a"]),
                };

                let timestamp = result["ts"]
                    .as_str()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or_else(|| Utc::now().timestamp_millis());

                let time = chrono::DateTime::from_timestamp_millis(timestamp)
                    .unwrap_or_else(|| Utc::now());

                let order_book = book_data.canonicalize(time);

                Ok(MarketEvent::from((
                    ExchangeId::BybitPerpetualsUsd,
                    subscription.instrument.key().clone(),
                    OrderBookEvent::Snapshot(order_book),
                )))
            }
        });

        try_join_all(l2_snapshot_futures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fnv::FnvHashMap;
    use rust_decimal_macros::dec;

    async fn init_transformer() -> BybitPerpetualsUsdOrderBooksL2Transformer<String> {
        let sub_id = SubscriptionId::from("orderbook|BTCUSDT");
        let map = Map(FnvHashMap::from_iter([(sub_id, "BTCUSDT".to_string())]));
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        BybitPerpetualsUsdOrderBooksL2Transformer::init(map, &[], tx)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_snapshot_and_update() {
        let mut transformer = init_transformer().await;

        let snapshot_json = r#"{
            "topic": "orderbook.BTCUSDT",
            "type": "snapshot",
            "ts": 1672304486868,
            "data": {
                "s": "BTCUSDT",
                "bids": [{"price":"16578.50","size":"0.5"}],
                "asks": [{"price":"16579.00","size":"0.4"}]
            }
        }"#;

        let snapshot: BybitPayload<BybitOrderBookL2Data> =
            serde_json::from_str(snapshot_json).unwrap();
        let events = transformer.transform(snapshot);
        let event = events.into_iter().next().unwrap().unwrap();
        match event.kind {
            OrderBookEvent::Snapshot(book) => {
                assert_eq!(book.bids().levels()[0].price, dec!(16578.50));
                assert_eq!(book.asks().levels()[0].price, dec!(16579.00));
            }
            _ => panic!("expected snapshot"),
        }

        let update_json = r#"{
            "topic": "orderbook.BTCUSDT",
            "type": "delta",
            "ts": 1672304486869,
            "data": {
                "s": "BTCUSDT",
                "bids": [{"price":"16578.50","size":"0.1"}],
                "asks": [{"price":"16579.50","size":"0.6"}]
            }
        }"#;

        let update: BybitPayload<BybitOrderBookL2Data> =
            serde_json::from_str(update_json).unwrap();
        let events = transformer.transform(update);
        let event = events.into_iter().next().unwrap().unwrap();
        match event.kind {
            OrderBookEvent::Update(book) => {
                assert_eq!(book.bids().levels()[0].price, dec!(16578.50));
                assert_eq!(book.asks().levels()[0].price, dec!(16579.50));
            }
            _ => panic!("expected update"),
        }
    }

    #[tokio::test]
    async fn test_out_of_order_and_reconnect() {
        let mut transformer = init_transformer().await;

        let update1_json = r#"{"topic":"orderbook.BTCUSDT","type":"delta","ts":2,"data":{"s":"BTCUSDT","bids":[],"asks":[]}}"#;
        let update2_json = r#"{"topic":"orderbook.BTCUSDT","type":"delta","ts":1,"data":{"s":"BTCUSDT","bids":[],"asks":[]}}"#;

        let u1: BybitPayload<BybitOrderBookL2Data> = serde_json::from_str(update1_json).unwrap();
        let u2: BybitPayload<BybitOrderBookL2Data> = serde_json::from_str(update2_json).unwrap();

        let t1 = transformer.transform(u1)[0].as_ref().unwrap().time_exchange;
        let t2 = transformer.transform(u2)[0].as_ref().unwrap().time_exchange;
        assert!(t1 > t2);

        let mut transformer = init_transformer().await;
        let events = transformer.transform(u2);
        assert!(matches!(events[0].as_ref().unwrap().kind, OrderBookEvent::Update(_)));
    }
}

fn parse_bybit_levels(levels_json: &serde_json::Value) -> Vec<BybitOrderBookLevel> {
    let mut levels = Vec::new();

    if let Some(level_array) = levels_json.as_array() {
        for level in level_array {
            if let (Some(price_str), Some(size_str)) = (level[0].as_str(), level[1].as_str()) {
                if let (Ok(price), Ok(size)) = (price_str.parse::<f64>(), size_str.parse::<f64>()) {
                    levels.push(BybitOrderBookLevel { price, size });
                }
            }
        }
    }

    levels
}

#[derive(Debug)]
pub struct BybitPerpetualsUsdOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<InstrumentKey>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<BybitPerpetualsUsd, InstrumentKey, OrderBooksL2>
    for BybitPerpetualsUsdOrderBooksL2Transformer<InstrumentKey>
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

impl<InstrumentKey> Transformer for BybitPerpetualsUsdOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = BybitPayload<BybitOrderBookL2Data>;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Extract subscription ID (it's already parsed by the deserializer)
        let subscription_id = input.subscription_id;

        // Find Instrument associated with Input
        let instrument_key = match self.instrument_map.find(&subscription_id) {
            Ok(key) => key.clone(),
            Err(unidentifiable) => return vec![Err(DataError::from(unidentifiable))],
        };

        // Use the timestamp from the message
        let time_exchange = input.time;

        // Create the OrderBook using the canonicalization process
        let order_book = input.data.canonicalize(time_exchange);

        // Use 'type' field to determine if this is a snapshot or update
        let event_type = if input.r#type == "snapshot" {
            OrderBookEvent::Snapshot(order_book)
        } else {
            OrderBookEvent::Update(order_book)
        };

        vec![Ok(MarketEvent {
            time_exchange,
            time_received: Utc::now(),
            exchange: ExchangeId::BybitPerpetualsUsd,
            instrument: instrument_key,
            kind: event_type,
        })]
    }
}
