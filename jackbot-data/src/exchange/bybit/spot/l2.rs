use super::super::book::{BybitOrderBookL2Data, BybitOrderBookLevel};
use crate::{
    Identifier, SnapshotFetcher,
    error::DataError,
    event::MarketEvent,
    exchange::bybit::{market::BybitMarket, message::BybitPayload, spot::BybitSpot},
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

/// Bybit HTTP OrderBook L2 snapshot URL
/// See docs: https://bybit-exchange.github.io/docs/v5/market/orderbook
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BYBIT_SPOT: &str = "https://api.bybit.com/v5/market/orderbook";

#[derive(Debug)]
pub struct BybitSpotOrderBooksL2SnapshotFetcher;

impl SnapshotFetcher<BybitSpot, OrderBooksL2> for BybitSpotOrderBooksL2SnapshotFetcher {
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<BybitSpot, Instrument, OrderBooksL2>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>>
    + Send
    where
        Instrument: InstrumentData,
        Subscription<BybitSpot, Instrument, OrderBooksL2>: Identifier<BybitMarket>,
    {
        let l2_snapshot_futures = subscriptions.iter().map(|subscription| {
            // Construct initial OrderBook snapshot GET url
            let market = subscription.id();
            let snapshot_url = format!(
                "{}?category=spot&symbol={}&limit=100",
                HTTP_BOOK_L2_SNAPSHOT_URL_BYBIT_SPOT, market.0,
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

                let order_book = book_data.normalize(time);

                Ok(MarketEvent::from((
                    ExchangeId::BybitSpot,
                    subscription.instrument.key().clone(),
                    OrderBookEvent::Snapshot(order_book),
                )))
            }
        });

        try_join_all(l2_snapshot_futures)
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
pub struct BybitSpotOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<InstrumentKey>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<BybitSpot, InstrumentKey, OrderBooksL2>
    for BybitSpotOrderBooksL2Transformer<InstrumentKey>
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

impl<InstrumentKey> Transformer for BybitSpotOrderBooksL2Transformer<InstrumentKey>
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

        // Create the OrderBook
        let order_book = input.data.normalize(time_exchange);

        // Use 'type' field to determine if this is a snapshot or update
        let event_type = if input.r#type == "snapshot" {
            OrderBookEvent::Snapshot(order_book)
        } else {
            OrderBookEvent::Update(order_book)
        };

        vec![Ok(MarketEvent {
            time_exchange,
            time_received: Utc::now(),
            exchange: ExchangeId::BybitSpot,
            instrument: instrument_key,
            kind: event_type,
        })]
    }
}
