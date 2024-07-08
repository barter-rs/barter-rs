use super::super::book::{l2::BinanceOrderBookL2Snapshot, BinanceLevel};
use crate::{
    error::DataError,
    subscription::book::OrderBook,
    transformer::book::{InstrumentOrderBook, OrderBookUpdater},
    Identifier,
};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    model::{instrument::Instrument, SubscriptionId},
    protocol::websocket::WsMessage,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// [`BinanceSpot`](super::BinanceSpot) HTTP OrderBook L2 snapshot url.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#order-book>
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_SPOT: &str = "https://api.binance.com/api/v3/depth";

/// [`BinanceSpot`](super::BinanceSpot) OrderBook Level2 deltas WebSocket message.
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
pub struct BinanceSpotOrderBookL2Delta {
    #[serde(
        alias = "s",
        deserialize_with = "super::super::book::l2::de_ob_l2_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "U")]
    pub first_update_id: u64,
    #[serde(alias = "u")]
    pub last_update_id: u64,
    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,
    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

impl Identifier<Option<SubscriptionId>> for BinanceSpotOrderBookL2Delta {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

/// [`Binance`](super::super::Binance) [`BinanceServerSpot`](super::BinanceServerSpot)
/// [`OrderBookUpdater`].
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
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceSpotBookUpdater {
    pub updates_processed: u64,
    pub last_update_id: u64,
    pub prev_last_update_id: u64,
}

impl BinanceSpotBookUpdater {
    /// Construct a new BinanceSpot [`OrderBookUpdater`] using the provided last_update_id from
    /// a HTTP snapshot.
    pub fn new(last_update_id: u64) -> Self {
        Self {
            updates_processed: 0,
            prev_last_update_id: last_update_id,
            last_update_id,
        }
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
        update: &BinanceSpotOrderBookL2Delta,
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
        update: &BinanceSpotOrderBookL2Delta,
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

#[async_trait]
impl OrderBookUpdater for BinanceSpotBookUpdater {
    type OrderBook = OrderBook;
    type Update = BinanceSpotOrderBookL2Delta;

    async fn init<Exchange, Kind>(
        _: mpsc::UnboundedSender<WsMessage>,
        instrument: Instrument,
    ) -> Result<InstrumentOrderBook<Instrument, Self>, DataError>
    where
        Exchange: Send,
        Kind: Send,
    {
        // Construct initial OrderBook snapshot GET url
        let snapshot_url = format!(
            "{}?symbol={}{}&limit=100",
            HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_SPOT,
            instrument.base.as_ref().to_uppercase(),
            instrument.quote.as_ref().to_uppercase()
        );

        // Fetch initial OrderBook snapshot via HTTP
        let snapshot = reqwest::get(snapshot_url)
            .await
            .map_err(SocketError::Http)?
            .json::<BinanceOrderBookL2Snapshot>()
            .await
            .map_err(SocketError::Http)?;

        Ok(InstrumentOrderBook {
            instrument,
            updater: Self::new(snapshot.last_update_id),
            book: OrderBook::from(snapshot),
        })
    }

    fn update(
        &mut self,
        book: &mut Self::OrderBook,
        update: Self::Update,
    ) -> Result<Option<Self::OrderBook>, DataError> {
        // BinanceSpot: How To Manage A Local OrderBook Correctly
        // See Self's Rust Docs for more information on each numbered step
        // See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>

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

        // Update OrderBook metadata & Levels:
        // 7. The data in each event is the absolute quantity for a price level.
        // 8. If the quantity is 0, remove the price level.
        book.last_update_time = Utc::now();
        book.bids.upsert(update.bids);
        book.asks.upsert(update.asks);

        // Update OrderBookUpdater metadata
        self.updates_processed += 1;
        self.prev_last_update_id = self.last_update_id;
        self.last_update_id = update.last_update_id;

        Ok(Some(book.snapshot()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_binance_spot_order_book_l2_delta() {
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
                serde_json::from_str::<BinanceSpotOrderBookL2Delta>(input).unwrap(),
                BinanceSpotOrderBookL2Delta {
                    subscription_id: SubscriptionId::from("@depth@100ms|ETHUSDT"),
                    first_update_id: 22611425143,
                    last_update_id: 22611425151,
                    bids: vec![
                        BinanceLevel {
                            price: 1209.67000000,
                            amount: 85.48210000
                        },
                        BinanceLevel {
                            price: 1209.66000000,
                            amount: 20.68790000
                        },
                    ],
                    asks: vec![]
                }
            );
        }
    }

    mod binance_spot_book_updater {
        use super::*;
        use crate::subscription::book::{Level, OrderBookSide};
        use barter_integration::model::Side;

        #[test]
        fn test_is_first_update() {
            struct TestCase {
                input: BinanceSpotBookUpdater,
                expected: bool,
            }

            let tests = vec![
                TestCase {
                    // TC0: is first update
                    input: BinanceSpotBookUpdater::new(10),
                    expected: true,
                },
                TestCase {
                    // TC1: is not first update
                    input: BinanceSpotBookUpdater {
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
        fn test_validate_first_update() {
            struct TestCase {
                updater: BinanceSpotBookUpdater,
                input: BinanceSpotOrderBookL2Delta,
                expected: Result<(), DataError>,
            }

            let tests = vec![
                TestCase {
                    // TC0: valid first update
                    updater: BinanceSpotBookUpdater {
                        updates_processed: 0,
                        last_update_id: 100,
                        prev_last_update_id: 90,
                    },
                    input: BinanceSpotOrderBookL2Delta {
                        subscription_id: SubscriptionId::from("subscription_id"),
                        first_update_id: 100,
                        last_update_id: 110,
                        bids: vec![],
                        asks: vec![],
                    },
                    expected: Ok(()),
                },
                TestCase {
                    // TC1: invalid first update w/ U > lastUpdateId+1
                    updater: BinanceSpotBookUpdater {
                        updates_processed: 0,
                        last_update_id: 100,
                        prev_last_update_id: 90,
                    },
                    input: BinanceSpotOrderBookL2Delta {
                        subscription_id: SubscriptionId::from("subscription_id"),
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
                    updater: BinanceSpotBookUpdater {
                        updates_processed: 0,
                        last_update_id: 100,
                        prev_last_update_id: 90,
                    },
                    input: BinanceSpotOrderBookL2Delta {
                        subscription_id: SubscriptionId::from("subscription_id"),
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
                    updater: BinanceSpotBookUpdater {
                        updates_processed: 0,
                        last_update_id: 100,
                        prev_last_update_id: 90,
                    },
                    input: BinanceSpotOrderBookL2Delta {
                        subscription_id: SubscriptionId::from("subscription_id"),
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
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }

        #[test]
        fn test_validate_next_update() {
            struct TestCase {
                updater: BinanceSpotBookUpdater,
                input: BinanceSpotOrderBookL2Delta,
                expected: Result<(), DataError>,
            }

            let tests = vec![
                TestCase {
                    // TC0: valid next update
                    updater: BinanceSpotBookUpdater {
                        updates_processed: 100,
                        last_update_id: 100,
                        prev_last_update_id: 100,
                    },
                    input: BinanceSpotOrderBookL2Delta {
                        subscription_id: SubscriptionId::from("subscription_id"),
                        first_update_id: 101,
                        last_update_id: 110,
                        bids: vec![],
                        asks: vec![],
                    },
                    expected: Ok(()),
                },
                TestCase {
                    // TC1: invalid first update w/ U != prev_last_update_id+1
                    updater: BinanceSpotBookUpdater {
                        updates_processed: 100,
                        last_update_id: 100,
                        prev_last_update_id: 90,
                    },
                    input: BinanceSpotOrderBookL2Delta {
                        subscription_id: SubscriptionId::from("subscription_id"),
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
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }

        #[test]
        fn update() {
            struct TestCase {
                updater: BinanceSpotBookUpdater,
                book: OrderBook,
                input_update: BinanceSpotOrderBookL2Delta,
                expected: Result<Option<OrderBook>, DataError>,
            }

            let time = Utc::now();

            let tests = vec![
                TestCase {
                    // TC0: Drop any event where u is <= lastUpdateId in the snapshot
                    updater: BinanceSpotBookUpdater {
                        updates_processed: 100,
                        last_update_id: 100,
                        prev_last_update_id: 0,
                    },
                    book: OrderBook {
                        last_update_time: time,
                        bids: OrderBookSide::new(Side::Buy, vec![Level::new(50, 1)]),
                        asks: OrderBookSide::new(Side::Sell, vec![Level::new(100, 1)]),
                    },
                    input_update: BinanceSpotOrderBookL2Delta {
                        subscription_id: SubscriptionId::from("subscription_id"),
                        first_update_id: 0,
                        last_update_id: 100, // u == updater.lastUpdateId
                        bids: vec![],
                        asks: vec![],
                    },
                    expected: Ok(None),
                },
                TestCase {
                    // TC1: valid update with sorted snapshot generated
                    updater: BinanceSpotBookUpdater {
                        updates_processed: 100,
                        last_update_id: 100,
                        prev_last_update_id: 100,
                    },
                    book: OrderBook {
                        last_update_time: time,
                        bids: OrderBookSide::new(
                            Side::Buy,
                            vec![Level::new(80, 1), Level::new(100, 1), Level::new(90, 1)],
                        ),
                        asks: OrderBookSide::new(
                            Side::Sell,
                            vec![Level::new(150, 1), Level::new(110, 1), Level::new(120, 1)],
                        ),
                    },
                    input_update: BinanceSpotOrderBookL2Delta {
                        subscription_id: SubscriptionId::from("subscription_id"),
                        first_update_id: 101,
                        last_update_id: 110,
                        bids: vec![
                            // Level exists & new value is 0 => remove Level
                            BinanceLevel {
                                price: 80.0,
                                amount: 0.0,
                            },
                            // Level exists & new value is > 0 => replace Level
                            BinanceLevel {
                                price: 90.0,
                                amount: 10.0,
                            },
                        ],
                        asks: vec![
                            // Level does not exist & new value > 0 => insert new Level
                            BinanceLevel {
                                price: 200.0,
                                amount: 1.0,
                            },
                            // Level does not exist & new value is 0 => no change
                            BinanceLevel {
                                price: 500.0,
                                amount: 0.0,
                            },
                        ],
                    },
                    expected: Ok(Some(OrderBook {
                        last_update_time: time,
                        bids: OrderBookSide::new(
                            Side::Buy,
                            vec![Level::new(100, 1), Level::new(90, 10)],
                        ),
                        asks: OrderBookSide::new(
                            Side::Sell,
                            vec![
                                Level::new(110, 1),
                                Level::new(120, 1),
                                Level::new(150, 1),
                                Level::new(200, 1),
                            ],
                        ),
                    })),
                },
            ];

            for (index, mut test) in tests.into_iter().enumerate() {
                let actual = test.updater.update(&mut test.book, test.input_update);

                match (actual, test.expected) {
                    (Ok(Some(actual)), Ok(Some(expected))) => {
                        // Replace time with deterministic timestamp
                        let actual = OrderBook {
                            last_update_time: time,
                            ..actual
                        };
                        assert_eq!(actual, expected, "TC{} failed", index)
                    }
                    (Ok(None), Ok(None)) => {
                        // Test passed
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
    }
}
