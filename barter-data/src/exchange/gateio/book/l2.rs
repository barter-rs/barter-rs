use super::{super::channel::GateioChannel, GateioLevel};
use crate::exchange::Connector;
use crate::exchange::gateio::Gateio;
use crate::exchange::gateio::spot::GateioServerSpot;
use crate::{
    Identifier, SnapshotFetcher,
    books::OrderBook,
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::{
        gateio::{market::GateioMarket, message::GateioMessage, spot::GateioSpot},
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
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    Transformer, error::SocketError, protocol::websocket::WsMessage, subscription::SubscriptionId,
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

/// [`GateioSpot`] HTTP OrderBook L2 snapshot url.
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/en/#retrieve-order-book>
pub const HTTP_BOOK_L2_SNAPSHOT_URL_GATEIO_SPOT: &str =
    "https://api.gateio.ws/api/v4/spot/order_book";

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct GateioOrderBookL2Snapshot {
    pub id: u64,
    #[serde(default, with = "chrono::serde::ts_milliseconds_option")]
    pub current: Option<DateTime<Utc>>,
    #[serde(default, with = "chrono::serde::ts_milliseconds_option")]
    pub update: Option<DateTime<Utc>>,
    pub bids: Vec<GateioLevel>,
    pub asks: Vec<GateioLevel>,
}
impl From<GateioOrderBookL2Snapshot> for OrderBookEvent {
    fn from(snapshot: GateioOrderBookL2Snapshot) -> Self {
        Self::Snapshot(OrderBook::new(
            snapshot.id,
            snapshot.update,
            snapshot.bids,
            snapshot.asks,
        ))
    }
}
impl<InstrumentKey> From<(ExchangeId, InstrumentKey, GateioOrderBookL2Snapshot)>
    for MarketEvent<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, snapshot): (ExchangeId, InstrumentKey, GateioOrderBookL2Snapshot),
    ) -> Self {
        let time_received = Utc::now();
        Self {
            time_exchange: snapshot.update.unwrap_or(time_received),
            time_received,
            exchange,
            instrument,
            kind: OrderBookEvent::from(snapshot),
        }
    }
}

#[derive(Debug)]
pub struct GateioSpotOrderBooksL2SnapshotFetcher;

impl SnapshotFetcher<Gateio<GateioServerSpot>, OrderBooksL2>
    for GateioSpotOrderBooksL2SnapshotFetcher
{
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<Gateio<GateioServerSpot>, Instrument, OrderBooksL2>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>>
    + Send
    where
        Instrument: InstrumentData,
        Subscription<Gateio<GateioServerSpot>, Instrument, OrderBooksL2>: Identifier<GateioMarket>,
    {
        let l2_snapshot_futures = subscriptions.iter().map(|sub| {
            // Construct initial OrderBook snapshot GET url
            let market = sub.id();
            let snapshot_url = format!(
                "{}?currency_pair={}&limit=100&with_id=true",
                HTTP_BOOK_L2_SNAPSHOT_URL_GATEIO_SPOT,
                market.as_ref(),
            );

            async move {
                // Fetch initial OrderBook snapshot via HTTP
                let snapshot = reqwest::get(snapshot_url)
                    .await
                    .map_err(SocketError::Http)?
                    .json::<GateioOrderBookL2Snapshot>()
                    .await
                    .map_err(SocketError::Http)?;

                Ok(MarketEvent::from((
                    ExchangeId::GateioSpot,
                    sub.instrument.key().clone(),
                    snapshot,
                )))
            }
        });

        try_join_all(l2_snapshot_futures)
    }
}

#[derive(Debug, Constructor, Deserialize)]
pub struct GateioOrderBookL2Meta<InstrumentKey, Sequencer> {
    pub key: InstrumentKey,
    pub sequencer: Sequencer,
}

#[derive(Debug, Deserialize)]
pub struct GateioSpotOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<GateioOrderBookL2Meta<InstrumentKey, GateioSpotOrderBookL2Sequencer>>,
}

#[async_trait]
impl<InstrumentKey, Server> ExchangeTransformer<Gateio<Server>, InstrumentKey, OrderBooksL2>
    for GateioSpotOrderBooksL2Transformer<InstrumentKey>
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

                let sequencer = GateioSpotOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_update_id: snapshot.sequence,
                };

                Ok((
                    sub_id,
                    GateioOrderBookL2Meta::new(instrument_key, sequencer),
                ))
            })
            .collect::<Result<Map<_>, _>>()?;

        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for GateioSpotOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = GateioOrderBookL2;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Find Instrument associated with Input and transform
        let instrument = match self.instrument_map.find_mut(&subscription_id) {
            Ok(instrument) => instrument,
            Err(unidentifiable) => return vec![Err(DataError::from(unidentifiable))],
        };

        // Drop any outdated updates & validate sequence for relevant updates
        let valid_update = match instrument.sequencer.validate_sequence(input) {
            Ok(Some(valid_update)) => valid_update,
            Ok(None) => return vec![],
            Err(error) => return vec![Err(error)],
        };

        MarketIter::<InstrumentKey, OrderBookEvent>::from((
            GateioSpot::ID,
            instrument.key.clone(),
            valid_update,
        ))
        .0
    }
}

#[derive(Debug, Deserialize)]
pub struct GateioSpotOrderBookL2Sequencer {
    pub updates_processed: u64,
    pub last_update_id: u64,
}

impl GateioSpotOrderBookL2Sequencer {
    /// Construct a new [`Self`] with the provided initial snapshot `last_update_id`.

    /// How to maintain local order book:
    ///
    /// Subscribe spot.order_book_update, e.g. ["BTC_USDT", "100ms"] pushes update in BTC_USDT order book every 100ms
    /// Cache WebSocket notifications. Every notification use U and u to tell the first and last update ID since last notification.
    /// Retrieve base order book using REST API, and make sure the order book ID is recorded(referred as baseID below) e.g. https://api.gateio.ws/api/v4/spot/order_book?currency_pair=BTC_USDT&limit=100&with_id=true retrieves the full base order book of BTC_USDT
    /// Iterate the cached WebSocket notifications, and find the first one which the baseID falls into, i.e. U <= baseId+1 and u >= baseId+1, then start consuming from it. Note that amount in notifications are all absolute values. Use them to replace original value in corresponding price. If amount equals to 0, delete the price from the order book.
    /// Dump all notifications which satisfy u < baseID+1. If baseID+1 < first notification U, it means current base order book falls behind notifications. Start from step 3 to retrieve newer base order book.
    /// If any subsequent notification which satisfy U > baseID+1 is found, it means some updates are lost. Reconstruct local order book from step 3.
    ///
    /// /// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#order-book-channel>
    pub fn validate_sequence(
        &mut self,
        update: GateioOrderBookL2,
    ) -> Result<Option<GateioOrderBookL2>, DataError> {
        if update.data.first_update_id <= self.last_update_id + 1
            && update.data.last_update_id >= self.last_update_id + 1
        {
            // Update metadata
            self.updates_processed += 1;
            self.last_update_id = update.data.last_update_id;

            return Ok(Some(update));
        }

        if update.data.last_update_id < self.last_update_id + 1 {
            return Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.data.first_update_id,
            });
        }

        if update.data.first_update_id > self.last_update_id + 1 {
            return Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.data.first_update_id,
            });
        }
        return Err(DataError::InvalidSequence {
            prev_last_update_id: self.last_update_id,
            first_update_id: update.data.first_update_id,
        });
    }
}

pub type GateioOrderBookL2 = GateioMessage<GateioOrderBookL2Update>;

/// [`Gateio`](super::super::Gateio) OrderBook Level2 snapshot HTTP message.
///
/// Used as the starting [`OrderBook`] before OrderBook Level2 delta WebSocket updates are
/// applied.
///
/// ### Payload Examples
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#changed-order-book-levels>
/// #### GateioSpot GateioOrderBookL2Update
/// ```json
/// {
///    "t": 1606294781123,
///    "full": true,
///    "l": "100",
///    "e": "depthUpdate",
///    "E": 1606294781,
///    "s": "BTC_USDT",
///    "U": 48776301,
///    "u": 48776306,
///    "b": [
///      ["19137.74", "0.0001"],
///      ["19088.37", "0"]
///    ],
///    "a": [
///      ["19137.75", "0.6135"]
///    ]
///   }
/// }
/// ```

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct GateioOrderBookL2Update {
    #[serde(alias = "s", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(default, rename = "t", with = "chrono::serde::ts_milliseconds_option")]
    pub time_engine: Option<DateTime<Utc>>,
    pub full: Option<bool>,
    #[serde(rename = "U")]
    pub first_update_id: u64,
    #[serde(rename = "u")]
    pub last_update_id: u64,
    pub l: String,
    #[serde(rename = "b")]
    pub bids: Vec<GateioLevel>,
    #[serde(rename = "a")]
    pub asks: Vec<GateioLevel>,
}

impl Identifier<Option<SubscriptionId>> for GateioOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.data.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, GateioOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, message): (ExchangeId, InstrumentKey, GateioOrderBookL2),
    ) -> Self {
        let time_received = Utc::now();
        Self(vec![Ok(MarketEvent {
            time_exchange: message.data.time_engine.unwrap_or(time_received),
            time_received: time_received,
            exchange,
            instrument,
            kind: OrderBookEvent::from(message),
        })])
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, GateioOrderBookL2)>
    for MarketEvent<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, snapshot): (ExchangeId, InstrumentKey, GateioOrderBookL2),
    ) -> Self {
        let time_received = Utc::now();
        Self {
            time_exchange: snapshot.data.time_engine.unwrap_or(time_received),
            time_received,
            exchange,
            instrument,
            kind: OrderBookEvent::from(snapshot),
        }
    }
}

impl From<GateioOrderBookL2> for OrderBookEvent {
    fn from(snapshot: GateioOrderBookL2) -> Self {
        Self::Snapshot(OrderBook::new(
            snapshot.data.last_update_id,
            snapshot.data.time_engine,
            snapshot.data.bids,
            snapshot.data.asks,
        ))
    }
}

pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((GateioChannel::ORDER_BOOK_L2, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use rust_decimal_macros::dec;

        #[test]
        fn test_gateio_order_book_l2_snapshot() {
            struct TestCase {
                input: &'static str,
                expected: GateioOrderBookL2Update,
            }

            let tests = vec![TestCase {
                // TC0: valid Spot GateioOrderBookL2Update
                input: r#"
                    {   
                        "s":"BTC_USDT",
                        "full":true,
                        "u": 1027024,
                        "U": 48776301,
                        "l":"100",
                        "b": [
                            [
                                "4.00000000",
                                "431.00000000"
                            ]
                        ],
                        "a": [
                            [
                                "4.00000200",
                                "12.00000000"
                            ]
                        ]
                    }
                    "#,
                expected: GateioOrderBookL2Update {
                    full: Some(true),
                    first_update_id: 48776301,
                    subscription_id: SubscriptionId::from("spot.order_book_update|BTC_USDT"),
                    last_update_id: 1027024,
                    time_engine: Default::default(),
                    l: "100".to_string(),
                    bids: vec![GateioLevel {
                        price: dec!(4.00000000),
                        amount: dec!(431.00000000),
                    }],
                    asks: vec![GateioLevel {
                        price: dec!(4.00000200),
                        amount: dec!(12.00000000),
                    }],
                },
            }];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(
                    serde_json::from_str::<GateioOrderBookL2Update>(test.input).unwrap(),
                    test.expected,
                    "TC{} failed",
                    index
                );
            }
        }
    }
}
