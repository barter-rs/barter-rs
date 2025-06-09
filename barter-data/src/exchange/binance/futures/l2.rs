use super::super::book::BinanceLevel;
use crate::{
    Identifier, SnapshotFetcher,
    books::OrderBook,
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::{
        Connector,
        binance::{
            book::l2::{BinanceOrderBookL2Meta, BinanceOrderBookL2Snapshot},
            futures::BinanceFuturesUsd,
            market::BinanceMarket,
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
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    Transformer, error::SocketError, protocol::websocket::WsMessage, subscription::SubscriptionId,
};
use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use serde::{Deserialize, Serialize};
use std::future::Future;
use tokio::sync::mpsc::UnboundedSender;

/// [`BinanceFuturesUsd`] HTTP OrderBook L2 snapshot url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#order-book>
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_FUTURES_USD: &str =
    "https://fapi.binance.com/fapi/v1/depth";

#[derive(Debug)]
pub struct BinanceFuturesUsdOrderBooksL2SnapshotFetcher;

impl SnapshotFetcher<BinanceFuturesUsd, OrderBooksL2>
    for BinanceFuturesUsdOrderBooksL2SnapshotFetcher
{
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<BinanceFuturesUsd, Instrument, OrderBooksL2>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>>
    + Send
    where
        Instrument: InstrumentData,
        Subscription<BinanceFuturesUsd, Instrument, OrderBooksL2>: Identifier<BinanceMarket>,
    {
        let l2_snapshot_futures = subscriptions.iter().map(|sub| {
            // Construct initial OrderBook snapshot GET url
            let market = sub.id();
            let snapshot_url = format!(
                "{}?symbol={}&limit=100",
                HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_FUTURES_USD,
                market.as_ref(),
            );

            async move {
                // Fetch initial OrderBook snapshot via HTTP
                let snapshot = reqwest::get(snapshot_url)
                    .await
                    .map_err(SocketError::Http)?
                    .json::<BinanceOrderBookL2Snapshot>()
                    .await
                    .map_err(SocketError::Http)?;

                Ok(MarketEvent::from((
                    ExchangeId::BinanceFuturesUsd,
                    sub.instrument.key().clone(),
                    snapshot,
                )))
            }
        });

        try_join_all(l2_snapshot_futures)
    }
}

#[derive(Debug)]
pub struct BinanceFuturesUsdOrderBooksL2Transformer<InstrumentKey> {
    instrument_map:
        Map<BinanceOrderBookL2Meta<InstrumentKey, BinanceFuturesUsdOrderBookL2Sequencer>>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<BinanceFuturesUsd, InstrumentKey, OrderBooksL2>
    for BinanceFuturesUsdOrderBooksL2Transformer<InstrumentKey>
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

                let sequencer = BinanceFuturesUsdOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_update_id: snapshot.sequence(),
                };

                Ok((
                    sub_id,
                    BinanceOrderBookL2Meta::new(instrument_key, sequencer),
                ))
            })
            .collect::<Result<Map<_>, _>>()?;

        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for BinanceFuturesUsdOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = BinanceFuturesOrderBookL2Update;
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
            BinanceFuturesUsd::ID,
            instrument.key.clone(),
            valid_update,
        ))
        .0
    }
}

/// [`Binance`](super::Binance) [`BinanceServerFuturesUsd`](super::BinanceServerFuturesUsd)
/// [`BinanceFuturesUsdOrderBookL2Sequencer`].
///
/// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly
///
/// 1. Open a stream to wss://fstream.binance.com/stream?streams=BTCUSDT@depth.
/// 2. Buffer the events you receive from the stream.
/// 3. Get a depth snapshot from <https://fapi.binance.com/fapi/v1/depth?symbol=BTCUSDT&limit=1000>.
/// 4. -- *DIFFERENT FROM SPOT* --
///    Drop any event where u is < lastUpdateId in the snapshot.
/// 5. -- *DIFFERENT FROM SPOT* --
///    The first processed event should have U <= lastUpdateId AND u >= lastUpdateId
/// 6. -- *DIFFERENT FROM SPOT* --
///    While listening to the stream, each new event's pu should be equal to the previous
///    event's u, otherwise initialize the process from step 3.
/// 7. The data in each event is the absolute quantity for a price level.
/// 8. If the quantity is 0, remove the price level.
///
/// Notes:
///  - Receiving an event that removes a price level that is not in your local order book can happen and is normal.
///  - Uppercase U => first_update_id
///  - Lowercase u => last_update_id,
///  - Lowercase pu => prev_last_update_id
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#how-to-manage-a-local-order-book-correctly>
#[derive(Debug)]
pub struct BinanceFuturesUsdOrderBookL2Sequencer {
    pub updates_processed: u64,
    pub last_update_id: u64,
}

impl BinanceFuturesUsdOrderBookL2Sequencer {
    /// Construct a new [`Self`] with the provided initial snapshot `last_update_id`.
    pub fn new(last_update_id: u64) -> Self {
        Self {
            updates_processed: 0,
            last_update_id,
        }
    }

    /// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly
    /// See Self's Rust Docs for more information on each numbered step
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_sequence(
        &mut self,
        update: BinanceFuturesOrderBookL2Update,
    ) -> Result<Option<BinanceFuturesOrderBookL2Update>, DataError> {
        // 4. Drop any event where u is < lastUpdateId in the snapshot:
        if update.last_update_id < self.last_update_id {
            return Ok(None);
        }

        if self.is_first_update() {
            // 5. The first processed event should have U <= lastUpdateId AND u >= lastUpdateId:
            self.validate_first_update(&update)?;
        } else {
            // 6. Each new event's pu should be equal to the previous event's u:
            self.validate_next_update(&update)?;
        }

        // Update metadata
        self.updates_processed += 1;
        self.last_update_id = update.last_update_id;

        Ok(Some(update))
    }

    /// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly: Step 5:
    /// "The first processed event should have U <= lastUpdateId AND u >= lastUpdateId"
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#how-to-manage-a-local-order-book-correctly>
    pub fn is_first_update(&self) -> bool {
        self.updates_processed == 0
    }

    /// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly: Step 5:
    /// "The first processed event should have U <= lastUpdateId AND u >= lastUpdateId"
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_first_update(
        &self,
        update: &BinanceFuturesOrderBookL2Update,
    ) -> Result<(), DataError> {
        if update.first_update_id <= self.last_update_id
            && update.last_update_id >= self.last_update_id
        {
            Ok(())
        } else {
            Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.first_update_id,
            })
        }
    }

    /// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly: Step 6:
    /// "While listening to the stream, each new event's pu should be equal to the previous
    ///  event's u, otherwise initialize the process from step 3."
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_next_update(
        &self,
        update: &BinanceFuturesOrderBookL2Update,
    ) -> Result<(), DataError> {
        if update.prev_last_update_id == self.last_update_id {
            Ok(())
        } else {
            Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.first_update_id,
            })
        }
    }
}

/// [`BinanceFuturesUsd`] OrderBook Level2 deltas WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
/// ```json
/// {
///     "e": "depthUpdate",
///     "E": 123456789,
///     "T": 123456788,
///     "s": "BTCUSDT",
///     "U": 157,
///     "u": 160,
///     "pu": 149,
///     "b": [
///         ["0.0024", "10"]
///     ],
///     "a": [
///         ["0.0026", "100"]
///     ]
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceFuturesOrderBookL2Update {
    #[serde(
        alias = "s",
        deserialize_with = "super::super::book::l2::de_ob_l2_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    #[serde(
        alias = "E",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time_exchange: DateTime<Utc>,
    #[serde(
        alias = "T",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time_engine: DateTime<Utc>,
    #[serde(alias = "U")]
    pub first_update_id: u64,
    #[serde(alias = "u")]
    pub last_update_id: u64,
    #[serde(alias = "pu")]
    pub prev_last_update_id: u64,
    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,
    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

impl Identifier<Option<SubscriptionId>> for BinanceFuturesOrderBookL2Update {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BinanceFuturesOrderBookL2Update)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, update): (
            ExchangeId,
            InstrumentKey,
            BinanceFuturesOrderBookL2Update,
        ),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: update.time_exchange,
            time_received: Utc::now(),
            exchange,
            instrument,
            kind: OrderBookEvent::Update(OrderBook::new(
                update.last_update_id,
                Some(update.time_engine),
                update.bids,
                update.asks,
            )),
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::Level;
    use rust_decimal_macros::dec;

    #[test]
    fn test_de_binance_futures_order_book_l2_update() {
        let input = r#"
            {
                "e": "depthUpdate",
                "E": 1571889248277,
                "T": 1571889248276,
                "s": "BTCUSDT",
                "U": 157,
                "u": 160,
                "pu": 149,
                "b": [
                    [
                        "0.0024",
                        "10"
                    ]
                ],
                "a": [
                    [
                        "0.0026",
                        "100"
                    ]
                ]
            }
        "#;

        assert_eq!(
            serde_json::from_str::<BinanceFuturesOrderBookL2Update>(input).unwrap(),
            BinanceFuturesOrderBookL2Update {
                subscription_id: SubscriptionId::from("@depth@100ms|BTCUSDT"),
                time_exchange: DateTime::from_timestamp_millis(1571889248277).unwrap(),
                time_engine: DateTime::from_timestamp_millis(1571889248276).unwrap(),
                first_update_id: 157,
                last_update_id: 160,
                prev_last_update_id: 149,
                bids: vec![BinanceLevel {
                    price: dec!(0.0024),
                    amount: dec!(10.0)
                },],
                asks: vec![BinanceLevel {
                    price: dec!(0.0026),
                    amount: dec!(100.0)
                },]
            }
        );
    }

    #[test]
    fn test_sequencer_is_first_update() {
        struct TestCase {
            sequencer: BinanceFuturesUsdOrderBookL2Sequencer,
            expected: bool,
        }

        let tests = vec![
            TestCase {
                // TC0: is first update
                sequencer: BinanceFuturesUsdOrderBookL2Sequencer::new(10),
                expected: true,
            },
            TestCase {
                // TC1: is not first update
                sequencer: BinanceFuturesUsdOrderBookL2Sequencer {
                    updates_processed: 10,
                    last_update_id: 100,
                },
                expected: false,
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            assert_eq!(
                test.sequencer.is_first_update(),
                test.expected,
                "TC{} failed",
                index
            );
        }
    }

    #[test]
    fn test_sequencer_validate_first_update() {
        struct TestCase {
            updater: BinanceFuturesUsdOrderBookL2Sequencer,
            input: BinanceFuturesOrderBookL2Update,
            expected: Result<(), DataError>,
        }

        let tests = vec![
            TestCase {
                // TC0: valid first update
                updater: BinanceFuturesUsdOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_update_id: 100,
                },
                input: BinanceFuturesOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    time_engine: Default::default(),
                    first_update_id: 100,
                    last_update_id: 110,
                    prev_last_update_id: 90,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Ok(()),
            },
            TestCase {
                // TC1: invalid first update w/ u < lastUpdateId
                updater: BinanceFuturesUsdOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_update_id: 100,
                },
                input: BinanceFuturesOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    time_engine: Default::default(),
                    first_update_id: 100,
                    last_update_id: 90,
                    prev_last_update_id: 90,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Err(DataError::InvalidSequence {
                    prev_last_update_id: 100,
                    first_update_id: 100,
                }),
            },
            TestCase {
                // TC2: invalid first update w/ U > lastUpdateId
                updater: BinanceFuturesUsdOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_update_id: 100,
                },
                input: BinanceFuturesOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    time_engine: Default::default(),
                    first_update_id: 110,
                    last_update_id: 120,
                    prev_last_update_id: 90,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Err(DataError::InvalidSequence {
                    prev_last_update_id: 100,
                    first_update_id: 110,
                }),
            },
            TestCase {
                // TC3: invalid first update w/  u < lastUpdateId & U > lastUpdateId
                updater: BinanceFuturesUsdOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_update_id: 100,
                },
                input: BinanceFuturesOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    time_engine: Default::default(),
                    first_update_id: 110,
                    last_update_id: 90,
                    prev_last_update_id: 90,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Err(DataError::InvalidSequence {
                    prev_last_update_id: 100,
                    first_update_id: 110,
                }),
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = test.updater.validate_first_update(&test.input);
            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!(
                        "TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                    );
                }
            }
        }
    }

    #[test]
    fn test_sequencer_validate_next_update() {
        struct TestCase {
            updater: BinanceFuturesUsdOrderBookL2Sequencer,
            input: BinanceFuturesOrderBookL2Update,
            expected: Result<(), DataError>,
        }

        let tests = vec![
            TestCase {
                // TC0: valid next update
                updater: BinanceFuturesUsdOrderBookL2Sequencer {
                    updates_processed: 100,
                    last_update_id: 100,
                },
                input: BinanceFuturesOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    time_engine: Default::default(),
                    first_update_id: 101,
                    last_update_id: 110,
                    prev_last_update_id: 100,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Ok(()),
            },
            TestCase {
                // TC1: invalid first update w/ pu != prev_last_update_id
                updater: BinanceFuturesUsdOrderBookL2Sequencer {
                    updates_processed: 100,
                    last_update_id: 100,
                },
                input: BinanceFuturesOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    time_engine: Default::default(),
                    first_update_id: 100,
                    last_update_id: 90,
                    prev_last_update_id: 90,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Err(DataError::InvalidSequence {
                    prev_last_update_id: 100,
                    first_update_id: 100,
                }),
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = test.updater.validate_next_update(&test.input);
            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!(
                        "TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                    );
                }
            }
        }
    }

    #[test]
    fn test_update_barter_order_book_with_sequenced_updates() {
        struct TestCase {
            sequencer: BinanceFuturesUsdOrderBookL2Sequencer,
            book: OrderBook,
            input_update: BinanceFuturesOrderBookL2Update,
            expected: OrderBook,
        }

        let tests = vec![
            TestCase {
                // TC0: Drop any event where u is < lastUpdateId in the snapshot
                sequencer: BinanceFuturesUsdOrderBookL2Sequencer {
                    updates_processed: 100,
                    last_update_id: 100,
                },
                book: OrderBook::new(0, None, vec![Level::new(50, 1)], vec![Level::new(100, 1)]),
                input_update: BinanceFuturesOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    time_engine: Default::default(),
                    first_update_id: 0,
                    last_update_id: 0,
                    prev_last_update_id: 0,
                    bids: vec![],
                    asks: vec![],
                },
                expected: OrderBook::new(
                    0,
                    None,
                    vec![Level::new(50, 1)],
                    vec![Level::new(100, 1)],
                ),
            },
            TestCase {
                // TC1: valid update & relevant update
                sequencer: BinanceFuturesUsdOrderBookL2Sequencer {
                    updates_processed: 100,
                    last_update_id: 100,
                },
                book: OrderBook::new(
                    100,
                    None,
                    vec![Level::new(80, 1), Level::new(100, 1), Level::new(90, 1)],
                    vec![Level::new(150, 1), Level::new(110, 1), Level::new(120, 1)],
                ),
                input_update: BinanceFuturesOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    time_engine: Default::default(),
                    first_update_id: 101,
                    last_update_id: 110,
                    prev_last_update_id: 100,
                    bids: vec![
                        // Level exists & new value is 0 => remove Level
                        BinanceLevel {
                            price: dec!(80),
                            amount: dec!(0),
                        },
                        // Level exists & new value is > 0 => replace Level
                        BinanceLevel {
                            price: dec!(90),
                            amount: dec!(10),
                        },
                    ],
                    asks: vec![
                        // Level does not exist & new value > 0 => insert new Level
                        BinanceLevel {
                            price: dec!(200),
                            amount: dec!(1),
                        },
                        // Level does not exist & new value is 0 => no change
                        BinanceLevel {
                            price: dec!(500),
                            amount: dec!(0),
                        },
                    ],
                },
                expected: OrderBook::new(
                    110,
                    None,
                    vec![Level::new(100, 1), Level::new(90, 10)],
                    vec![
                        Level::new(110, 1),
                        Level::new(120, 1),
                        Level::new(150, 1),
                        Level::new(200, 1),
                    ],
                ),
            },
        ];

        for (index, mut test) in tests.into_iter().enumerate() {
            if let Some(valid_update) = test.sequencer.validate_sequence(test.input_update).unwrap()
            {
                let barter_update = OrderBookEvent::Update(OrderBook::new(
                    valid_update.last_update_id,
                    None,
                    valid_update.bids,
                    valid_update.asks,
                ));

                test.book.update(&barter_update);
            }

            assert_eq!(test.book, test.expected, "TC{index} failed");
        }
    }
}
