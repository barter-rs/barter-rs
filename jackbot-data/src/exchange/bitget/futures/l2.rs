//! Level 2 order book (full book) types and parsing for Bitget futures.

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
        bitget::{channel::BitgetChannel, futures::BitgetFutures, market::BitgetMarket},
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

/// Bitget HTTP OrderBook L2 snapshot url for futures market.
/// See: https://bitgetlimited.github.io/apidoc/en/mix/#get-orderbook
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BITGET_FUTURES: &str =
    "https://api.bitget.com/api/mix/v1/market/depth";

/// Sequencer implementation for Bitget Futures order book.
#[derive(Debug, Clone)]
pub struct BitgetFuturesOrderBookL2Sequencer {
    pub last_update_id: u64,
    pub updates_processed: u64,
}

impl L2Sequencer<BitgetOrderBookL2Update> for BitgetFuturesOrderBookL2Sequencer {
    fn new(last_update_id: u64) -> Self {
        Self {
            last_update_id,
            updates_processed: 0,
        }
    }

    fn validate_sequence(
        &mut self,
        update: BitgetOrderBookL2Update,
    ) -> Result<Option<BitgetOrderBookL2Update>, DataError> {
        // Bitget doesn't provide sequence numbers for futures either - just apply all updates
        self.updates_processed += 1;
        Ok(Some(update))
    }

    fn is_first_update(&self) -> bool {
        self.updates_processed == 0
    }
}

#[derive(Debug)]
pub struct BitgetFuturesOrderBooksL2SnapshotFetcher;

impl SnapshotFetcher<BitgetFutures, OrderBooksL2> for BitgetFuturesOrderBooksL2SnapshotFetcher {
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<BitgetFutures, Instrument, OrderBooksL2>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>>
    + Send
    where
        Instrument: InstrumentData,
        Subscription<BitgetFutures, Instrument, OrderBooksL2>: Identifier<BitgetMarket>,
    {
        let l2_snapshot_futures = subscriptions.iter().map(|subscription| {
            // Construct initial OrderBook snapshot GET url
            let market = subscription.id();
            // In futures API we need to provide both symbol and contract type (linear/inverse)
            // For simplicity, assuming "umcbl" (linear) as default
            let symbol = market.0;
            let snapshot_url = format!(
                "{}?symbol={}&limit=150&productType=umcbl",
                HTTP_BOOK_L2_SNAPSHOT_URL_BITGET_FUTURES, symbol,
            );

            async move {
                // Fetch initial OrderBook snapshot via HTTP
                let snapshot = reqwest::get(snapshot_url)
                    .await
                    .map_err(SocketError::Http)?
                    .json::<BitgetOrderBookL2SnapshotResponse>()
                    .await
                    .map_err(SocketError::Http)?;

                Ok(MarketEvent::from((
                    ExchangeId::BitgetFutures,
                    subscription.instrument.key().clone(),
                    snapshot.data,
                )))
            }
        });

        try_join_all(l2_snapshot_futures)
    }
}

/// Response wrapper for Bitget API responses
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BitgetOrderBookL2SnapshotResponse {
    pub code: String,
    pub msg: String,
    pub data: BitgetOrderBookL2Snapshot,
}

/// Bitget OrderBook L2 snapshot for futures
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BitgetOrderBookL2Snapshot {
    #[serde(rename = "asks")]
    pub asks: Vec<(Decimal, Decimal)>,
    #[serde(rename = "bids")]
    pub bids: Vec<(Decimal, Decimal)>,
    #[serde(rename = "timestamp")]
    pub timestamp: u64,
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BitgetOrderBookL2Snapshot)>
    for MarketEvent<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, snapshot): (ExchangeId, InstrumentKey, BitgetOrderBookL2Snapshot),
    ) -> Self {
        let bids = snapshot
            .bids
            .iter()
            .map(|(p, a)| Level::new(*p, *a))
            .collect();
        let asks = snapshot
            .asks
            .iter()
            .map(|(p, a)| Level::new(*p, *a))
            .collect();
        let sequence = snapshot.timestamp; // Using timestamp as sequence

        Self {
            time_exchange: Utc::now(), // No exchange time in snapshot, using received time
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: OrderBookEvent::Snapshot(OrderBook::new(sequence, None, bids, asks)),
        }
    }
}

#[derive(Debug)]
pub struct BitgetFuturesOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<(InstrumentKey, BitgetFuturesOrderBookL2Sequencer)>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<BitgetFutures, InstrumentKey, OrderBooksL2>
    for BitgetFuturesOrderBooksL2Transformer<InstrumentKey>
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

                let sequencer = <BitgetFuturesOrderBookL2Sequencer as L2Sequencer<
                    BitgetOrderBookL2Update,
                >>::new(snapshot.sequence);

                Ok((sub_id, (instrument_key, sequencer)))
            })
            .collect::<Result<Map<_>, _>>()?;

        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for BitgetFuturesOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = BitgetOrderBookL2Update;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Find Instrument associated with Input and transform
        let (instrument_key, sequencer) = match self.instrument_map.find_mut(&subscription_id) {
            Ok(instrument) => instrument,
            Err(unidentifiable) => return vec![Err(DataError::from(unidentifiable))],
        };

        // Validate update sequence
        let valid_update = match sequencer.validate_sequence(input) {
            Ok(Some(valid_update)) => valid_update,
            Ok(None) => return vec![],
            Err(error) => return vec![Err(error)],
        };

        MarketIter::<InstrumentKey, OrderBookEvent>::from((
            BitgetFutures::ID,
            instrument_key.clone(),
            valid_update,
        ))
        .0
    }
}

/// Bitget real-time OrderBook Level2 update for futures.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BitgetOrderBookL2Update {
    #[serde(alias = "instId", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(default = "Utc::now")]
    pub time: DateTime<Utc>,
    #[serde(alias = "bids")]
    pub bids: Vec<(Decimal, Decimal)>,
    #[serde(alias = "asks")]
    pub asks: Vec<(Decimal, Decimal)>,
    #[serde(alias = "action")]
    pub action: String, // "snapshot" or "update"
    #[serde(default)]
    pub timestamp: u64,
}

impl Identifier<Option<SubscriptionId>> for BitgetOrderBookL2Update {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BitgetOrderBookL2Update)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, update): (ExchangeId, InstrumentKey, BitgetOrderBookL2Update),
    ) -> Self {
        let bids = update
            .bids
            .iter()
            .map(|(p, a)| Level::new(*p, *a))
            .collect();
        let asks = update
            .asks
            .iter()
            .map(|(p, a)| Level::new(*p, *a))
            .collect();

        let event = if update.action == "snapshot" {
            OrderBookEvent::Snapshot(OrderBook::new(update.timestamp, None, bids, asks))
        } else {
            OrderBookEvent::Update(OrderBook::new(update.timestamp, None, bids, asks))
        };

        Self(vec![Ok(MarketEvent {
            time_exchange: update.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: event,
        })])
    }
}

impl HasUpdateIds for BitgetOrderBookL2Update {
    fn first_update_id(&self) -> u64 {
        self.timestamp
    }

    fn last_update_id(&self) -> u64 {
        self.timestamp
    }
}

/// Deserialize a BitgetOrderBookL2 "instId" as the associated SubscriptionId.
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BitgetChannel::ORDER_BOOK_L2, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_bitget_futures_order_book_l2_update() {
        let input = r#"{
            "action": "update",
            "instId": "BTCUSDT_UMCBL",
            "bids": [["30000.0", "1.0"]],
            "asks": [["30010.0", "2.0"]]
        }"#;
        let update: BitgetOrderBookL2Update = serde_json::from_str(input).unwrap();
        assert_eq!(update.bids[0], (dec!(30000.0), dec!(1.0)));
        assert_eq!(update.asks[0], (dec!(30010.0), dec!(2.0)));
        assert_eq!(update.action, "update");
    }

    #[test]
    fn test_bitget_futures_order_book_l2_snapshot() {
        let input = r#"{
            "action": "snapshot",
            "instId": "BTCUSDT_UMCBL",
            "bids": [["30000.0", "1.0"]],
            "asks": [["30010.0", "2.0"]]
        }"#;
        let snapshot: BitgetOrderBookL2Update = serde_json::from_str(input).unwrap();
        assert_eq!(snapshot.bids[0], (dec!(30000.0), dec!(1.0)));
        assert_eq!(snapshot.asks[0], (dec!(30010.0), dec!(2.0)));
        assert_eq!(snapshot.action, "snapshot");
    }
}
