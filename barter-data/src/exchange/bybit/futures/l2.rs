use super::super::book::{l2::BybitOrderBookL2, BybitLevel};
use crate::{
    error::DataError,
    exchange::{bybit::channel::BybitChannel, subscription::ExchangeSub},
    subscription::book::{OrderBook, OrderBookSide},
    transformer::book::{InstrumentOrderBook, OrderBookUpdater},
    Identifier,
};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    model::{instrument::Instrument, Side, SubscriptionId},
    protocol::websocket::WsMessage,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// [`BybitPerpetualsUsd`](super::BybitPerpetualsUsd) HTTP OrderBook L2 snapshot url.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/market/orderbook>
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BYBIT: &str = "https://api.bybit.com/v5/market/orderbook";

/// [`BybitPerpetualsUsd`](super::BybitPerpetualsUsd) OrderBook Level2 deltas WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://bybit-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
/// ```json
/// {
///     "topic": "orderbook.50.BTCUSDT",
///     "type": "delta",
///     "ts": 1687940967466,
///     "data": {
///         "s": "BTCUSDT",
///         "b": [
///              ["30240.30", "1.305"],
///              ["30240.00", "0"]
///         ],
///         "a": [
///             ["30248.70", "0"],
///             ["30249.30", "0.892"],
///         ],
///         "u": 177400507,
///         "seq": 66544703342
///     }
///     "cts": 1687940967464
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitPerpetualsOrderBookL2 {
    #[serde(alias = "topic")]
    pub topic: String,
    #[serde(alias = "type")]
    pub r#type: BybitPerpetualsOrderBookL2Type,
    #[serde(
        alias = "ts",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc",
        default = "Utc::now"
    )]
    pub time: DateTime<Utc>,
    #[serde(alias = "data")]
    pub data: BybitPerpetualsOrderBookL2Data,
    #[serde(
        alias = "cts",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc",
        default = "Utc::now"
    )]
    pub created_time: DateTime<Utc>,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum BybitPerpetualsOrderBookL2Type {
    #[serde(alias = "delta")]
    Delta,
    #[serde(alias = "snapshot")]
    Snapshot,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitPerpetualsOrderBookL2Data {
    #[serde(alias = "s", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "b")]
    pub bids: Vec<BybitLevel>,
    #[serde(alias = "a")]
    pub asks: Vec<BybitLevel>,
    #[serde(alias = "u")]
    pub update_id: u64,
    #[serde(alias = "seq")]
    pub sequence: u64,
}

impl Identifier<Option<SubscriptionId>> for BybitPerpetualsOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.data.subscription_id.clone())
    }
}

/// [`Bybit`](super::super::Bybit) [`BybitServerFuturesUsd`](super::BybitServerFuturesUsd)
/// [`OrderBookUpdater`].
///
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitPerpetualsBookUpdater {
    pub last_update_id: u64,
    pub last_sequence: u64,
}

impl BybitPerpetualsBookUpdater {
    /// Construct a new BybitPerpetuals [`OrderBookUpdater`] using the provided last_update_id from
    /// a HTTP snapshot.
    pub fn new(last_update_id: u64, last_sequence: u64) -> Self {
        Self {
            last_update_id,
            last_sequence,
        }
    }

    pub fn validate_next_update(
        &self,
        update: &BybitPerpetualsOrderBookL2,
    ) -> Result<(), DataError> {
        if update.r#type == BybitPerpetualsOrderBookL2Type::Snapshot {
            return Ok(());
        }
        if update.data.update_id == self.last_update_id + 1 {
            Ok(())
        } else {
            Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.data.update_id, // TODO
            })
        }
    }
}

#[async_trait]
impl OrderBookUpdater for BybitPerpetualsBookUpdater {
    type OrderBook = OrderBook;
    type Update = BybitPerpetualsOrderBookL2;

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
            "{}?category=linear&symbol={}{}&limit=100",
            HTTP_BOOK_L2_SNAPSHOT_URL_BYBIT,
            instrument.base.as_ref().to_uppercase(),
            instrument.quote.as_ref().to_uppercase()
        );

        // Fetch initial OrderBook snapshot via HTTP
        let snapshot = reqwest::get(snapshot_url)
            .await
            .map_err(SocketError::Http)?
            .json::<BybitOrderBookL2>()
            .await
            .map_err(SocketError::Http)?;

        Ok(InstrumentOrderBook {
            instrument,
            updater: Self::new(snapshot.result.last_update_id, snapshot.result.sequence),
            book: OrderBook::from(snapshot),
        })
    }

    fn update(
        &mut self,
        book: &mut Self::OrderBook,
        update: Self::Update,
    ) -> Result<Option<Self::OrderBook>, DataError> {
        if update.data.update_id < self.last_update_id {
            return Ok(None);
        }

        self.validate_next_update(&update)?;

        if update.r#type == BybitPerpetualsOrderBookL2Type::Snapshot {
            // Replace OrderBook with snapshot
            book.last_update_time = Utc::now();
            book.bids = OrderBookSide::new(Side::Buy, update.data.bids);
            book.asks = OrderBookSide::new(Side::Sell, update.data.asks);
        } else {
            // Update OrderBook metadata & Levels:
            // The data in each event is the absolute quantity for a price level.
            // If the quantity is 0, remove the price level.
            book.last_update_time = Utc::now();
            book.bids.upsert(update.data.bids);
            book.asks.upsert(update.data.asks);
        }

        // Update OrderBookUpdater metadata
        self.last_update_id = update.data.update_id;
        self.last_sequence = update.data.sequence;

        Ok(Some(book.snapshot()))
    }
}

/// Deserialize a [`BybitOrderBookL2`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`].
///
/// eg/ "orderbook.50.BTCUSDT"
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BybitChannel::ORDER_BOOK_L2, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use std::time::Duration;

        use barter_integration::de::datetime_utc_from_epoch_duration;

        use super::*;

        #[test]
        fn test_bybit_futures_order_book_l2_deltas() {
            let input = r#"
            {
                "topic": "orderbook.50.BTCUSDT",
                "type": "delta",
                "ts": 1687940967466,
                "data": {
                    "s": "BTCUSDT",
                    "b": [
                        ["30247.20", "30.028"],
                        ["30245.40", "0.224"],
                        ["30242.10", "1.593"],
                        ["30240.30", "1.305"],
                        ["30240.00", "0"]
                    ],
                    "a": [
                        ["30248.70", "0"],
                        ["30249.30", "0.892"],
                        ["30249.50", "1.778"],
                        ["30249.60", "0"],
                        ["30251.90", "2.947"],
                        ["30252.20", "0.659"],
                        ["30252.50", "4.591"]
                    ],
                    "u": 177400507,
                    "seq": 66544703342
                },
                "cts": 1687940967464
            }
        "#;

        let time = datetime_utc_from_epoch_duration(Duration::from_millis(1687940967466));
        let created_time = datetime_utc_from_epoch_duration(Duration::from_millis(1687940967464));

            assert_eq!(
                serde_json::from_str::<BybitPerpetualsOrderBookL2>(input).unwrap(),
                BybitPerpetualsOrderBookL2 {
                    topic: "orderbook.50.BTCUSDT".to_string(),
                    r#type: BybitPerpetualsOrderBookL2Type::Delta,
                    time,
                    data: BybitPerpetualsOrderBookL2Data {
                        subscription_id: SubscriptionId::from("orderbook.50|BTCUSDT"),
                        bids: vec![
                            BybitLevel {
                                price: 30247.20,
                                amount: 30.028
                            },
                            BybitLevel {
                                price: 30245.40,
                                amount: 0.224
                            },
                            BybitLevel {
                                price: 30242.10,
                                amount: 1.593
                            },
                            BybitLevel {
                                price: 30240.30,
                                amount: 1.305
                            },
                            BybitLevel {
                                price: 30240.00,
                                amount: 0.0
                            }
                        ],
                        asks: vec![
                            BybitLevel {
                                price: 30248.70,
                                amount: 0.0
                            },
                            BybitLevel {
                                price: 30249.30,
                                amount: 0.892
                            },
                            BybitLevel {
                                price: 30249.50,
                                amount: 1.778
                            },
                            BybitLevel {
                                price: 30249.60,
                                amount: 0.0
                            },
                            BybitLevel {
                                price: 30251.90,
                                amount: 2.947
                            },
                            BybitLevel {
                                price: 30252.20,
                                amount: 0.659
                            },
                            BybitLevel {
                                price: 30252.50,
                                amount: 4.591
                            }
                        ],
                        update_id: 177400507,
                        sequence: 66544703342
                    },
                    created_time,
                }
            );
        }
    }

    mod bybit_futures_book_updater {
        use super::*;
        use crate::subscription::book::{Level, OrderBookSide};
        use barter_integration::model::Side;

        #[test]
        fn test_validate_next_update() {
            struct TestCase {
                updater: BybitPerpetualsBookUpdater,
                input: BybitPerpetualsOrderBookL2,
                expected: Result<(), DataError>,
            }

            let time = Utc::now();

            let tests = vec![
                TestCase {
                    // TC0: valid next update
                    updater: BybitPerpetualsBookUpdater {
                        last_update_id: 100,
                        last_sequence: 0,
                    },
                    input: BybitPerpetualsOrderBookL2 {
                        topic: "orderbook.50.BTCUSDT".to_string(),
                        r#type: BybitPerpetualsOrderBookL2Type::Delta,
                        time,
                        data: BybitPerpetualsOrderBookL2Data {
                            subscription_id: SubscriptionId::from("orderbook.50|BTCUSDT"),
                            bids: vec![],
                            asks: vec![],
                            update_id: 101,
                            sequence: 66544703342,
                        },
                        created_time: time,
                    },
                    expected: Ok(()),
                },
                TestCase {
                    // TC1: invalid update_id
                    updater: BybitPerpetualsBookUpdater {
                        last_update_id: 100,
                        last_sequence: 0,
                    },
                    input: BybitPerpetualsOrderBookL2 {
                        topic: "orderbook.50.BTCUSDT".to_string(),
                        r#type: BybitPerpetualsOrderBookL2Type::Delta,
                        time,
                        data: BybitPerpetualsOrderBookL2Data {
                            subscription_id: SubscriptionId::from("orderbook.50|BTCUSDT"),
                            bids: vec![],
                            asks: vec![],
                            update_id: 100,
                            sequence: 66544703342,
                        },
                        created_time: time,
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
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }

        #[test]
        fn update() {
            struct TestCase {
                updater: BybitPerpetualsBookUpdater,
                book: OrderBook,
                input_update: BybitPerpetualsOrderBookL2,
                expected: Result<Option<OrderBook>, DataError>,
            }

            let time = Utc::now();

            let tests = vec![
                TestCase {
                    // TC0: Drop any event where u is < lastUpdateId in the snapshot
                    updater: BybitPerpetualsBookUpdater {
                        last_update_id: 100,
                        last_sequence: 0,
                    },
                    book: OrderBook {
                        last_update_time: time,
                        bids: OrderBookSide::new(Side::Buy, vec![Level::new(50, 1)]),
                        asks: OrderBookSide::new(Side::Sell, vec![Level::new(100, 1)]),
                    },
                    input_update: BybitPerpetualsOrderBookL2 {
                        topic: "orderbook.50.BTCUSDT".to_string(),
                        r#type: BybitPerpetualsOrderBookL2Type::Delta,
                        time,
                        data: BybitPerpetualsOrderBookL2Data {
                            subscription_id: SubscriptionId::from("BTCUSDT"),
                            bids: vec![],
                            asks: vec![],
                            update_id: 99,
                            sequence: 66544703342,
                        },
                        created_time: time,
                    },
                    expected: Ok(None),
                },
                TestCase {
                    // TC1: valid update with sorted snapshot generated
                    updater: BybitPerpetualsBookUpdater {
                        last_update_id: 100,
                        last_sequence: 0,
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
                    input_update: BybitPerpetualsOrderBookL2 {
                        topic: "orderbook.50.BTCUSDT".to_string(),
                        r#type: BybitPerpetualsOrderBookL2Type::Snapshot,
                        time,
                        data: BybitPerpetualsOrderBookL2Data {
                            subscription_id: SubscriptionId::from("BTCUSDT"),
                            bids: vec![
                                BybitLevel {
                                    price: 100.0,
                                    amount: 1.0,
                                },
                                BybitLevel {
                                    price: 90.0,
                                    amount: 10.0,
                                },
                            ],
                            asks: vec![
                                BybitLevel {
                                    price: 110.0,
                                    amount: 1.0,
                                },
                                BybitLevel {
                                    price: 120.0,
                                    amount: 1.0,
                                },
                                BybitLevel {
                                    price: 150.0,
                                    amount: 1.0,
                                },
                                BybitLevel {
                                    price: 200.0,
                                    amount: 1.0,
                                },
                            ],
                            update_id: 100,
                            sequence: 66544703342,
                        },
                        created_time: time,
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
