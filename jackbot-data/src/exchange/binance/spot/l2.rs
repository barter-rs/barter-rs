use super::super::book::BinanceLevel;
use crate::{
    Identifier, SnapshotFetcher,
    books::OrderBook,
    books::l2_sequencer::{BinanceSpotOrderBookL2Sequencer, HasUpdateIds, L2Sequencer},
    books::{Canonicalizer, Level},
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::{
        Connector,
        binance::{
            book::l2::{BinanceOrderBookL2Meta, BinanceOrderBookL2Snapshot},
            market::BinanceMarket,
            spot::BinanceSpot,
        },
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
use serde::{Deserialize, Serialize};
use std::future::Future;
use tokio::sync::mpsc::UnboundedSender;

/// [`BinanceSpot`] HTTP OrderBook L2 snapshot url.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#order-book>
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_SPOT: &str = "https://api.binance.com/api/v3/depth";

#[derive(Debug)]
pub struct BinanceSpotOrderBooksL2SnapshotFetcher;

impl SnapshotFetcher<BinanceSpot, OrderBooksL2> for BinanceSpotOrderBooksL2SnapshotFetcher {
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<BinanceSpot, Instrument, OrderBooksL2>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>>
    + Send
    where
        Instrument: InstrumentData,
        Subscription<BinanceSpot, Instrument, OrderBooksL2>: Identifier<BinanceMarket>,
    {
        let l2_snapshot_futures = subscriptions.iter().map(|subscription| {
            // Construct initial OrderBook snapshot GET url
            let market = subscription.id();
            let snapshot_url = format!(
                "{}?symbol={}&limit=100",
                HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_SPOT, market.0,
            );

            async move {
                // Fetch initial OrderBook snapshot via HTTP
                let snapshot = reqwest::get(snapshot_url)
                    .await
                    .map_err(SocketError::Http)?
                    .json::<BinanceOrderBookL2Snapshot>()
                    .await
                    .map_err(SocketError::Http)?;

                let timestamp = Utc::now();
                let orderbook = snapshot.canonicalize(timestamp);
                Ok(MarketEvent::from((
                    ExchangeId::BinanceSpot,
                    subscription.instrument.key().clone(),
                    OrderBookEvent::Snapshot(orderbook),
                )))
            }
        });

        try_join_all(l2_snapshot_futures)
    }
}

#[derive(Debug)]
pub struct BinanceSpotOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<BinanceOrderBookL2Meta<InstrumentKey, BinanceSpotOrderBookL2Sequencer>>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<BinanceSpot, InstrumentKey, OrderBooksL2>
    for BinanceSpotOrderBooksL2Transformer<InstrumentKey>
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

                let book_meta = BinanceOrderBookL2Meta::new(
                    instrument_key,
                    <BinanceSpotOrderBookL2Sequencer as L2Sequencer<
                        BinanceSpotOrderBookL2Update,
                    >>::new(snapshot.sequence),
                );

                Ok((sub_id, book_meta))
            })
            .collect::<Result<Map<_>, _>>()?;

        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for BinanceSpotOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = BinanceSpotOrderBookL2Update;
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
            BinanceSpot::ID,
            instrument.key.clone(),
            valid_update,
        ))
        .0
    }
}

/// [`Binance`](super::Binance) [`BinanceServerSpot`](super::BinanceServerSpot)
/// [`BinanceSpotOrderBookL2Sequencer`].
///
/// BinanceSpot: How To Manage A Local OrderBook Correctly
///
/// 1. Open a stream to wss://stream.binance.com:9443/ws/BTCUSDT@depth.
/// 2. Buffer the events you receive from the stream.
/// 3. Get a depth snapshot from <https://api.binance.com/api/v3/depth?symbol=BNBBTC&limit=1000>.
/// 4. -- *DIFFERENT FROM FUTURES* --
///    Drop any event where u is <= lastUpdateId in the snapshot.
/// 5. -- *DIFFERENT FROM FUTURES* --
///    The first processed event should have U <= lastUpdateId+1 AND u >= lastUpdateId+1.
/// 6. -- *DIFFERENT FROM FUTURES* --
///    While listening to the stream, each new event's U should be equal to the
///    previous event's u+1, otherwise initialize the process from step 3.
/// 7. The data in each event is the absolute quantity for a price level.
/// 8. If the quantity is 0, remove the price level.
///
/// Notes:
///  - Receiving an event that removes a price level that is not in your local order book can happen and is normal.
///  - Uppercase U => first_update_id
///  - Lowercase u => last_update_id,
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceSpotOrderBookL2Update {
    #[serde(
        alias = "s",
        deserialize_with = "super::super::book::l2::de_ob_l2_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    #[serde(
        alias = "E",
        deserialize_with = "jackbot_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time_exchange: DateTime<Utc>,
    #[serde(alias = "U")]
    pub first_update_id: u64,
    #[serde(alias = "u")]
    pub last_update_id: u64,
    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,
    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

impl Identifier<Option<SubscriptionId>> for BinanceSpotOrderBookL2Update {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl Canonicalizer for BinanceSpotOrderBookL2Update {
    fn canonicalize(&self, timestamp: DateTime<Utc>) -> OrderBook {
        OrderBook::new(
            self.last_update_id,
            Some(timestamp),
            self.bids
                .iter()
                .map(|level| Level::new(level.price, level.amount)),
            self.asks
                .iter()
                .map(|level| Level::new(level.price, level.amount)),
        )
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BinanceSpotOrderBookL2Update)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, update): (
            ExchangeId,
            InstrumentKey,
            BinanceSpotOrderBookL2Update,
        ),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: update.time_exchange,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: OrderBookEvent::Update(update.canonicalize(update.time_exchange)),
        })])
    }
}

impl HasUpdateIds for BinanceSpotOrderBookL2Update {
    fn first_update_id(&self) -> u64 {
        self.first_update_id
    }
    fn last_update_id(&self) -> u64 {
        self.last_update_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::Level;
    use rust_decimal_macros::dec;

    #[test]
    fn test_de_binance_spot_order_book_l2_update() {
        let input = r#"{
            "e": "depthUpdate",
            "E": 1571889248277,
            "s": "BTCUSDT",
            "U": 157,
            "u": 160,
            "b": [["0.0024", "10"]],
            "a": [["0.0026", "100"]]
        }"#;

        assert_eq!(
            serde_json::from_str::<BinanceSpotOrderBookL2Update>(input).unwrap(),
            BinanceSpotOrderBookL2Update {
                subscription_id: SubscriptionId::from("@depth@100ms|BTCUSDT"),
                time_exchange: DateTime::from_timestamp_millis(1571889248277).unwrap(),
                first_update_id: 157,
                last_update_id: 160,
                bids: vec![BinanceLevel { price: dec!(0.0024), amount: dec!(10) }],
                asks: vec![BinanceLevel { price: dec!(0.0026), amount: dec!(100) }],
            }
        );
    }

    #[test]
    fn test_sequencer_is_first_update() {
        let tests = vec![
            (BinanceSpotOrderBookL2Sequencer::new(10), true),
            (
                BinanceSpotOrderBookL2Sequencer {
                    updates_processed: 1,
                    last_update_id: 100,
                    prev_last_update_id: 99,
                },
                false,
            ),
        ];

        for (seq, expected) in tests {
            assert_eq!(seq.is_first_update(), expected);
        }
    }

    #[test]
    fn test_sequencer_validate_first_update() {
        let mut sequencer = BinanceSpotOrderBookL2Sequencer::new(100);
        let valid = BinanceSpotOrderBookL2Update {
            subscription_id: SubscriptionId::from("id"),
            time_exchange: Default::default(),
            first_update_id: 100,
            last_update_id: 101,
            bids: vec![],
            asks: vec![],
        };
        assert!(sequencer.validate_first_update(&valid).is_ok());

        let invalid = BinanceSpotOrderBookL2Update { last_update_id: 90, ..valid.clone() };
        assert!(sequencer.validate_first_update(&invalid).is_err());
    }

    #[test]
    fn test_sequencer_validate_next_update() {
        let sequencer = BinanceSpotOrderBookL2Sequencer {
            updates_processed: 10,
            last_update_id: 100,
            prev_last_update_id: 100,
        };
        let ok_update = BinanceSpotOrderBookL2Update {
            subscription_id: SubscriptionId::from("id"),
            time_exchange: Default::default(),
            first_update_id: 101,
            last_update_id: 110,
            bids: vec![],
            asks: vec![],
        };
        assert!(sequencer.validate_next_update(&ok_update).is_ok());

        let bad_update = BinanceSpotOrderBookL2Update { first_update_id: 105, ..ok_update };
        assert!(sequencer.validate_next_update(&bad_update).is_err());
    }

    #[test]
    fn test_update_order_book_with_sequenced_updates() {
        let mut sequencer = BinanceSpotOrderBookL2Sequencer {
            updates_processed: 100,
            last_update_id: 100,
            prev_last_update_id: 99,
        };
        let mut book = OrderBook::new(100, None, vec![Level::new(80, 1)], vec![Level::new(120, 1)]);

        let update = BinanceSpotOrderBookL2Update {
            subscription_id: SubscriptionId::from("id"),
            time_exchange: Default::default(),
            first_update_id: 101,
            last_update_id: 110,
            bids: vec![BinanceLevel { price: dec!(80), amount: dec!(0) }],
            asks: vec![BinanceLevel { price: dec!(130), amount: dec!(1) }],
        };

        if let Some(valid_update) = sequencer.validate_sequence(update).unwrap() {
            let event = OrderBookEvent::Update(OrderBook::new(
                valid_update.last_update_id,
                None,
                valid_update.bids,
                valid_update.asks,
            ));
            book.update(event);
        }

        assert_eq!(book.sequence, 110);
        assert!(book.bids().levels().is_empty());
        assert_eq!(book.asks().levels()[1].price, dec!(130));
    }
}
