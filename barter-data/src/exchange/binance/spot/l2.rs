use super::super::book::BinanceLevel;
use crate::{
    books::OrderBook,
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::{
        binance::{
            book::l2::{BinanceOrderBookL2Meta, BinanceOrderBookL2Snapshot},
            market::BinanceMarket,
            spot::BinanceSpot,
        },
        Connector,
    },
    instrument::InstrumentData,
    subscription::{
        book::{OrderBookEvent, OrderBooksL2},
        Map, Subscription,
    },
    transformer::ExchangeTransformer,
    Identifier, SnapshotFetcher,
};
use async_trait::async_trait;
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    error::SocketError, protocol::websocket::WsMessage, subscription::SubscriptionId, Transformer,
};
use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
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

                Ok(MarketEvent::from((
                    ExchangeId::BinanceSpot,
                    subscription.instrument.key().clone(),
                    snapshot,
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
                    BinanceSpotOrderBookL2Sequencer::new(snapshot.sequence),
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

        println!(
            "Update: {}-{}",
            valid_update.first_update_id, valid_update.last_update_id
        );

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
#[derive(Debug)]
pub struct BinanceSpotOrderBookL2Sequencer {
    pub updates_processed: u64,
    pub last_update_id: u64,
    pub prev_last_update_id: u64,
}

impl BinanceSpotOrderBookL2Sequencer {
    /// Construct a new [`Self`] with the provided initial snapshot `last_update_id`.
    pub fn new(last_update_id: u64) -> Self {
        Self {
            updates_processed: 0,
            last_update_id,
            prev_last_update_id: last_update_id,
        }
    }

    /// BinanceSpot: How To Manage A Local OrderBook Correctly
    /// See Self's Rust Docs for more information on each numbered step
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_sequence(
        &mut self,
        update: BinanceSpotOrderBookL2Update,
    ) -> Result<Option<BinanceSpotOrderBookL2Update>, DataError> {
        // 4. Drop any event where u is <= lastUpdateId in the snapshot:
        if update.last_update_id <= self.last_update_id {
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
        self.prev_last_update_id = self.last_update_id;
        self.last_update_id = update.last_update_id;

        Ok(Some(update))
    }

    /// BinanceSpot: How To Manage A Local OrderBook Correctly: Step 5:
    /// "The first processed event should have U <= lastUpdateId+1 AND u >= lastUpdateId+1"
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
    pub fn is_first_update(&self) -> bool {
        self.updates_processed == 0
    }

    /// BinanceSpot: How To Manage A Local OrderBook Correctly: Step 5:
    /// "The first processed event should have U <= lastUpdateId+1 AND u >= lastUpdateId+1"
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_first_update(
        &self,
        update: &BinanceSpotOrderBookL2Update,
    ) -> Result<(), DataError> {
        let expected_next_id = self.last_update_id + 1;
        if update.first_update_id <= expected_next_id && update.last_update_id >= expected_next_id {
            Ok(())
        } else {
            Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.first_update_id,
            })
        }
    }

    /// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly: Step 6:
    /// "While listening to the stream, each new event's U should be equal to the
    ///  previous event's u+1, otherwise initialize the process from step 3."
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_next_update(
        &self,
        update: &BinanceSpotOrderBookL2Update,
    ) -> Result<(), DataError> {
        let expected_next_id = self.last_update_id + 1;
        if update.first_update_id == expected_next_id {
            Ok(())
        } else {
            Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.first_update_id,
            })
        }
    }
}

/// [`BinanceSpot`] OrderBook Level2 deltas WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#partial-book-depth-streams>
/// ```json
/// {
///     "e":"depthUpdate",
///     "E":1671656397761,
///     "s":"ETHUSDT",
///     "U":22611425143,
///     "u":22611425151,
///     "b":[
///         ["1209.67000000","85.48210000"],
///         ["1209.66000000","20.68790000"]
///     ],
///     "a":[]
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceSpotOrderBookL2Update {
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
            kind: OrderBookEvent::Update(OrderBook::new(
                update.last_update_id,
                None,
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
    fn test_de_binance_spot_order_book_l2_update() {
        let input = r#"
            {
                "e":"depthUpdate",
                "E":1671656397761,
                "s":"ETHUSDT",
                "U":22611425143,
                "u":22611425151,
                "b":[
                    ["1209.67000000","85.48210000"],
                    ["1209.66000000","20.68790000"]
                ],
                "a":[]
            }
            "#;

        assert_eq!(
            serde_json::from_str::<BinanceSpotOrderBookL2Update>(input).unwrap(),
            BinanceSpotOrderBookL2Update {
                subscription_id: SubscriptionId::from("@depth@100ms|ETHUSDT"),
                time_exchange: DateTime::from_timestamp_millis(1671656397761).unwrap(),
                first_update_id: 22611425143,
                last_update_id: 22611425151,
                bids: vec![
                    BinanceLevel {
                        price: dec!(1209.67000000),
                        amount: dec!(85.48210000)
                    },
                    BinanceLevel {
                        price: dec!(1209.66000000),
                        amount: dec!(20.68790000)
                    },
                ],
                asks: vec![]
            }
        );
    }

    #[test]
    fn test_sequencer_is_first_update() {
        struct TestCase {
            input: BinanceSpotOrderBookL2Sequencer,
            expected: bool,
        }

        let tests = vec![
            TestCase {
                // TC0: is first update
                input: BinanceSpotOrderBookL2Sequencer::new(10),
                expected: true,
            },
            TestCase {
                // TC1: is not first update
                input: BinanceSpotOrderBookL2Sequencer {
                    updates_processed: 10,
                    last_update_id: 100,
                    prev_last_update_id: 90,
                },
                expected: false,
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            assert_eq!(
                test.input.is_first_update(),
                test.expected,
                "TC{} failed",
                index
            );
        }
    }

    #[test]
    fn test_sequencer_validate_first_update() {
        struct TestCase {
            sequencer: BinanceSpotOrderBookL2Sequencer,
            input: BinanceSpotOrderBookL2Update,
            expected: Result<(), DataError>,
        }

        let tests = vec![
            TestCase {
                // TC0: valid first update
                sequencer: BinanceSpotOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_update_id: 100,
                    prev_last_update_id: 90,
                },
                input: BinanceSpotOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    first_update_id: 100,
                    last_update_id: 110,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Ok(()),
            },
            TestCase {
                // TC1: invalid first update w/ U > lastUpdateId+1
                sequencer: BinanceSpotOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_update_id: 100,
                    prev_last_update_id: 90,
                },
                input: BinanceSpotOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    first_update_id: 102,
                    last_update_id: 90,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Err(DataError::InvalidSequence {
                    prev_last_update_id: 100,
                    first_update_id: 102,
                }),
            },
            TestCase {
                // TC2: invalid first update w/ u < lastUpdateId+1
                sequencer: BinanceSpotOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_update_id: 100,
                    prev_last_update_id: 90,
                },
                input: BinanceSpotOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    first_update_id: 110,
                    last_update_id: 90,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Err(DataError::InvalidSequence {
                    prev_last_update_id: 100,
                    first_update_id: 110,
                }),
            },
            TestCase {
                // TC3: invalid first update w/  U > lastUpdateId+1 & u < lastUpdateId+1
                sequencer: BinanceSpotOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_update_id: 100,
                    prev_last_update_id: 90,
                },
                input: BinanceSpotOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    first_update_id: 110,
                    last_update_id: 90,
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
            let actual = test.sequencer.validate_first_update(&test.input);
            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }

    #[test]
    fn test_sequencer_validate_next_update() {
        struct TestCase {
            sequencer: BinanceSpotOrderBookL2Sequencer,
            input: BinanceSpotOrderBookL2Update,
            expected: Result<(), DataError>,
        }

        let tests = vec![
            TestCase {
                // TC0: valid next update
                sequencer: BinanceSpotOrderBookL2Sequencer {
                    updates_processed: 100,
                    last_update_id: 100,
                    prev_last_update_id: 100,
                },
                input: BinanceSpotOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    first_update_id: 101,
                    last_update_id: 110,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Ok(()),
            },
            TestCase {
                // TC1: invalid first update w/ U != prev_last_update_id+1
                sequencer: BinanceSpotOrderBookL2Sequencer {
                    updates_processed: 100,
                    last_update_id: 100,
                    prev_last_update_id: 90,
                },
                input: BinanceSpotOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    first_update_id: 120,
                    last_update_id: 130,
                    bids: vec![],
                    asks: vec![],
                },
                expected: Err(DataError::InvalidSequence {
                    prev_last_update_id: 100,
                    first_update_id: 120,
                }),
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = test.sequencer.validate_next_update(&test.input);
            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }

    #[test]
    fn test_update_barter_order_book_with_sequenced_updates() {
        struct TestCase {
            sequencer: BinanceSpotOrderBookL2Sequencer,
            book: OrderBook,
            input_update: BinanceSpotOrderBookL2Update,
            expected: OrderBook,
        }

        let tests = vec![
            TestCase {
                // TC0: Drop any event where u is <= lastUpdateId in the snapshot
                sequencer: BinanceSpotOrderBookL2Sequencer {
                    updates_processed: 100,
                    last_update_id: 100,
                    prev_last_update_id: 0,
                },
                book: OrderBook::new(100, None, vec![Level::new(50, 1)], vec![Level::new(100, 1)]),
                input_update: BinanceSpotOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    first_update_id: 0,
                    last_update_id: 100, // u == updater.lastUpdateId
                    bids: vec![],
                    asks: vec![],
                },
                expected: OrderBook::new(
                    100,
                    None,
                    vec![Level::new(50, 1)],
                    vec![Level::new(100, 1)],
                ),
            },
            TestCase {
                // TC1: valid & relevant update
                sequencer: BinanceSpotOrderBookL2Sequencer {
                    updates_processed: 100,
                    last_update_id: 100,
                    prev_last_update_id: 100,
                },
                book: OrderBook::new(
                    100,
                    None,
                    vec![Level::new(80, 1), Level::new(100, 1), Level::new(90, 1)],
                    vec![Level::new(150, 1), Level::new(110, 1), Level::new(120, 1)],
                ),
                input_update: BinanceSpotOrderBookL2Update {
                    subscription_id: SubscriptionId::from("subscription_id"),
                    time_exchange: Default::default(),
                    first_update_id: 101,
                    last_update_id: 110,
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

                test.book.update(barter_update);
            }

            assert_eq!(test.book, test.expected, "TC{index} failed");
        }
    }
}
