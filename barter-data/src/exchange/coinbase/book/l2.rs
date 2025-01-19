use std::fmt::Debug;
use std::future::Future;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use futures_util::future::try_join_all;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use barter_instrument::exchange::ExchangeId;
use barter_instrument::Side;
use barter_integration::error::SocketError;
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::subscription::SubscriptionId;
use barter_integration::Transformer;
use crate::books::{Level, OrderBook};
use crate::error::DataError;
use crate::event::{MarketEvent, MarketIter};
use crate::exchange::coinbase::Coinbase;
use crate::exchange::Connector;
use crate::{Identifier, SnapshotFetcher};
use crate::exchange::coinbase::market::CoinbaseMarket;
use crate::instrument::InstrumentData;
use crate::subscription::book::{OrderBookEvent, OrderBooksL2};
use crate::subscription::{Map, Subscription};
use crate::transformer::ExchangeTransformer;

pub const HTTP_PRODUCT_BOOK_SNAPSHOT_URL: &str = "https://api.exchange.coinbase.com/products";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CoinbaseOrderBookSnapshot {
    pub sequence: u64,
    pub bids: Vec<CoinbaseOrderBookSnapshotLevel>,
    pub asks: Vec<CoinbaseOrderBookSnapshotLevel>,
    #[serde(
        alias = "time"
    )]
    pub time_exchange: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CoinbaseOrderBookL2Snapshot {
    pub product_id: String,
    pub bids: Vec<CoinbaseOrderBookL2SnapshotLevel>,
    pub asks: Vec<CoinbaseOrderBookL2SnapshotLevel>,
}

#[derive(Debug)]
pub struct CoinbaseOrderBooksL2SnapshotFetcher;

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, CoinbaseOrderBookSnapshot)> for
MarketEvent<InstrumentKey, OrderBookEvent> {
    fn from(
        (exchange, instrument, snapshot): (
            ExchangeId,
            InstrumentKey,
            CoinbaseOrderBookSnapshot,
        ),
    ) -> Self {
        MarketEvent {
            time_exchange: snapshot.time_exchange,
            time_received: Utc::now(),
            exchange,
            instrument,
            kind: OrderBookEvent::Snapshot(OrderBook::new(
                snapshot.sequence,
                Some(snapshot.time_exchange),
                snapshot.bids,
                snapshot.asks,
            )),
        }
    }
}

impl From<CoinbaseOrderBookSnapshotLevel> for Level {
    fn from(change: CoinbaseOrderBookSnapshotLevel) -> Self {
        Self {
            price: change.price,
            amount: change.size,
        }
    }
}

impl From<CoinbaseOrderBookL2SnapshotLevel> for Level {
    fn from(change: CoinbaseOrderBookL2SnapshotLevel) -> Self {
        Self {
            price: change.price,
            amount: change.size,
        }
    }
}

impl FromIterator<CoinbaseOrderBookL2SnapshotLevel> for Vec<Level> {
    fn from_iter<T: IntoIterator<Item=CoinbaseOrderBookL2SnapshotLevel>>(iter: T) -> Self {
        iter.into_iter().map(Level::from).collect()
    }
}

impl SnapshotFetcher<Coinbase, OrderBooksL2>for CoinbaseOrderBooksL2SnapshotFetcher {
    fn fetch_snapshots<Instrument>(subscriptions: &[Subscription<Coinbase, Instrument, OrderBooksL2>])
        -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, OrderBookEvent>>, SocketError>>
        + Send
    where
        Instrument: InstrumentData,
        Subscription<Coinbase, Instrument, OrderBooksL2>: Identifier<CoinbaseMarket>
    {
        let l2_snapshot_futures = subscriptions.iter().map(|sub| {
            // Construct initial OrderBook snapshot GET url
            let market = sub.id();
            let snapshot_url = format!(
                "{url}/{product_id}/book?level=2",
                url = HTTP_PRODUCT_BOOK_SNAPSHOT_URL,
                product_id = market.as_ref(),
            );

            async move {
                let client = Client::new();

                // Fetch initial OrderBook snapshot via HTTP
                let snapshot = client
                    .get(&snapshot_url)
                    .header("User-Agent", "barter-rs")
                    .send()
                    .await
                    .map_err(SocketError::Http)?
                    .json::<CoinbaseOrderBookSnapshot>()
                    .await
                    .map_err(SocketError::Http)?;

                Ok(MarketEvent::from((
                    ExchangeId::Coinbase,
                    sub.instrument.key().clone(),
                    snapshot,
                )))
            }
        });

        try_join_all(l2_snapshot_futures)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CoinbaseOrderBookSnapshotLevel {
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub size: Decimal,
    pub num_orders: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CoinbaseOrderBookL2SnapshotLevel {
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub size: Decimal
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CoinbaseOrderBookL2Update {
    pub product_id: String,
    #[serde(
        alias = "time"
    )]
    pub time_exchange: DateTime<Utc>,
    pub changes: Vec<CoinbaseOrderBookL2Change>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum CoinbaseOrderBookL2Message {
    Snapshot(CoinbaseOrderBookL2Snapshot),
    Update(CoinbaseOrderBookL2Update),
}

impl Identifier<Option<SubscriptionId>> for CoinbaseOrderBookL2Message {
    fn id(&self) -> Option<SubscriptionId> {
        match self {
            CoinbaseOrderBookL2Message::Snapshot(snapshot) => Some(SubscriptionId::from(format!("level2_batch|{}",  snapshot.product_id.clone()))),
            CoinbaseOrderBookL2Message::Update(update) => Some(SubscriptionId::from(format!("level2_batch|{}",  update.product_id.clone()))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CoinbaseOrderBookL2Change {
    pub side: Side,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub size: Decimal,
}

#[derive(Debug, Constructor)]
pub struct CoinbaseOrderBookL2Meta<InstrumentKey, Sequencer> {
    pub key: InstrumentKey,
    pub sequencer: Sequencer,
}

#[derive(Debug, Default)]
pub struct CoinbaseOrderBookL2Sequencer {
    pub updates_processed: u64,
    pub last_updated_at: DateTime<Utc>
}

impl CoinbaseOrderBookL2Sequencer {
    pub fn new(last_updated_at: DateTime<Utc>) -> Self {
        Self {
            updates_processed: 0,
            last_updated_at
        }
    }

    pub fn validate_sequence(&mut self, update: CoinbaseOrderBookL2Update) -> Result<Option<CoinbaseOrderBookL2Update>, DataError> {
        if update.time_exchange <= self.last_updated_at {
            return Ok(None);
        }

        self.updates_processed += 1;
        self.last_updated_at = update.time_exchange;

        Ok(Some(update))
    }
}

#[derive(Debug)]
pub struct CoinbaseOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<CoinbaseOrderBookL2Meta<InstrumentKey, CoinbaseOrderBookL2Sequencer>>
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<Coinbase, InstrumentKey, OrderBooksL2> for CoinbaseOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync + Debug,
{
    async fn init(
        instrument_map: Map<InstrumentKey>,
        initial_snapshots: &[MarketEvent<InstrumentKey, OrderBookEvent>],
        _: UnboundedSender<WsMessage>
    ) -> Result<Self, DataError> {

        let instrument_map = instrument_map.0
            .into_iter()
            .map(|(sub_id, instrument_key)| {

                let snapshot = initial_snapshots
                    .iter()
                    .find(|snapshot| snapshot.instrument == instrument_key)
                    .ok_or_else(|| DataError::InitialSnapshotMissing(sub_id.clone()))?;

                let OrderBookEvent::Snapshot(_snapshot) = &snapshot.kind else {
                    return Err(DataError::InitialSnapshotInvalid(String::from(
                        "expected OrderBookEvent::Snapshot but found OrderBookEvent::Update",
                    )));
                };

                Ok(
                    (
                        sub_id,
                        CoinbaseOrderBookL2Meta {
                            key: instrument_key.clone(),
                            sequencer: CoinbaseOrderBookL2Sequencer::new(Utc::now())
                        }
                    )
                )
            })
            .collect::<Result<Map<_>, _>>()?;

        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for CoinbaseOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync + Debug
{
    type Error = DataError;
    type Input = CoinbaseOrderBookL2Message;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {

        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        let instrument = match self.instrument_map.find_mut(&subscription_id) {
            Ok(instrument) => instrument,
            Err(unidentifiable) => return vec![Err(DataError::from(unidentifiable))],
        };

        match input {
            CoinbaseOrderBookL2Message::Snapshot(snapshot) => {
                let snapshot = MarketEvent::from((
                    Coinbase::ID,
                    instrument.key.clone(),
                    snapshot,
                ));

                vec![Ok(snapshot)]
            }
            CoinbaseOrderBookL2Message::Update(update) => {
                let valid_update = match instrument.sequencer.validate_sequence(update) {
                    Ok(Some(valid_update)) => valid_update,
                    Ok(None) => return vec![],
                    Err(error) => return vec![Err(error)],
                };

                MarketIter::<InstrumentKey, OrderBookEvent>::from((
                    Coinbase::ID,
                    instrument.key.clone(),
                    valid_update,
                )).0
            }
        }
    }
}

impl From<CoinbaseOrderBookL2Change> for Level {
    fn from(change: CoinbaseOrderBookL2Change) -> Self {
        Self {
            price: change.price,
            amount: change.size,
        }
    }
}

impl FromIterator<CoinbaseOrderBookL2Change> for Vec<Level> {
    fn from_iter<T: IntoIterator<Item=CoinbaseOrderBookL2Change>>(iter: T) -> Self {
        iter.into_iter().map(Level::from).collect()
    }
}


impl<InstrumentKey> From<(ExchangeId, InstrumentKey, CoinbaseOrderBookL2Snapshot)> for
MarketEvent<InstrumentKey, OrderBookEvent> {
    fn from(
        (exchange, instrument, snapshot): (
            ExchangeId,
            InstrumentKey,
            CoinbaseOrderBookL2Snapshot,
        ),
    ) -> Self {
        MarketEvent {
            time_exchange: Utc::now(),
            time_received: Utc::now(),
            exchange,
            instrument,
            kind: OrderBookEvent::Snapshot(OrderBook::new(
                0,
                Some(Utc::now()),
                snapshot.bids,
                snapshot.asks,
            )),
        }
    }
}


impl<InstrumentKey> From<(ExchangeId, InstrumentKey, CoinbaseOrderBookL2Update)>
for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, update): (
            ExchangeId,
            InstrumentKey,
            CoinbaseOrderBookL2Update,
        ),
    ) -> Self {
        let (bids, asks): (Vec<_>, Vec<_>) = update.changes.into_iter().partition(|change| change.side == Side::Buy);

        Self(vec![Ok(MarketEvent {
            time_exchange: update.time_exchange,
            time_received: Utc::now(),
            exchange,
            instrument,
            kind: OrderBookEvent::Update(OrderBook::new(
                0,
                Some(update.time_exchange),
                bids,
                asks,
            )),
        })])
    }
}

fn get_book_snapshot_url(symbol: &str) -> String {
    format!("https://api.exchange.coinbase.com/products/{symbol}/book", symbol = symbol)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use rust_decimal::Decimal;
    use barter_instrument::Side;
    use crate::books::{Level, OrderBook};
    use crate::error::DataError;
    use crate::subscription::book::OrderBookEvent;
    use super::{get_book_snapshot_url, CoinbaseOrderBookL2Change, CoinbaseOrderBookL2Sequencer, CoinbaseOrderBookL2Snapshot, CoinbaseOrderBookL2SnapshotLevel, CoinbaseOrderBookL2Update};

    #[test]
    fn test_get_book_snapshot_url() {
        assert_eq!(get_book_snapshot_url("BTC-USD"), "https://api.exchange.coinbase.com/products/BTC-USD/book");
    }

    #[test]
    fn test_de_coinbase_order_book_l2_update() {
        let input = r#"
            {
                "product_id": "BTC-USD",
                "time": "2022-08-04T15:25:05.010758Z",
                "changes": [
                    ["buy", "10000.00", "0.01"],
                    ["sell", "10001.00", "0.01"]
                ]
            }
        "#;

        assert_eq!(serde_json::from_str::<CoinbaseOrderBookL2Update>(input).unwrap(), CoinbaseOrderBookL2Update {
            product_id: "BTC-USD".to_string(),
            time_exchange: chrono::DateTime::from_str("2022-08-04T15:25:05.010758Z").unwrap(),
            changes: vec![
                CoinbaseOrderBookL2Change {
                    side: Side::Buy,
                    price: Decimal::from_str("10000.00").unwrap(),
                    size: Decimal::from_str("0.01").unwrap()
                },
                CoinbaseOrderBookL2Change {
                    side: Side::Sell,
                    price: Decimal::from_str("10001.00").unwrap(),
                    size: Decimal::from_str("0.01").unwrap()
                },
            ]
        });
    }

    #[test]
    fn test_de_coinbase_order_book_l2_snapshot() {
        let input = r#"
            {
                "product_id": "BTC-USD",
                "bids": [
                    ["10000.00", "0.01"]
                ],
                "asks": [
                    ["10001.00", "0.01"]
                ]
            }
        "#;

        let expected = CoinbaseOrderBookL2Snapshot {
            product_id: "BTC-USD".to_string(),
            bids: vec![
                CoinbaseOrderBookL2SnapshotLevel {
                    price: Decimal::from_str("10000.00").unwrap(),
                    size: Decimal::from_str("0.01").unwrap()
                }
            ],
            asks: vec![
                CoinbaseOrderBookL2SnapshotLevel {
                    price: Decimal::from_str("10001.00").unwrap(),
                    size: Decimal::from_str("0.01").unwrap()
                }
            ]
        };

        let result: CoinbaseOrderBookL2Snapshot = serde_json::from_str(input).unwrap();
        assert_eq!(result, expected);
    }


    #[test]
    fn test_coinbase_sequencer_validate_first_update() {
        struct TestCase {
            sequencer: CoinbaseOrderBookL2Sequencer,
            input: CoinbaseOrderBookL2Update,
            expected: Result<Option<CoinbaseOrderBookL2Update>, DataError>,
        }

        let tests = vec![
            TestCase {
                // TC0: valid first update
                sequencer: CoinbaseOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_updated_at: chrono::DateTime::from_str("2022-08-04T15:25:05.010758Z").unwrap(),
                },
                input: CoinbaseOrderBookL2Update {
                    product_id: "BTC-USD".to_string(),
                    time_exchange: chrono::DateTime::from_str("2022-08-04T15:25:06.010758Z").unwrap(),
                    changes: vec![],
                },
                expected: Ok(Some(CoinbaseOrderBookL2Update {
                    product_id: "BTC-USD".to_string(),
                    time_exchange: chrono::DateTime::from_str("2022-08-04T15:25:06.010758Z").unwrap(),
                    changes: vec![],
                })),
            },
            TestCase {
                // TC1: invalid first update with time_exchange <= last_updated_at
                sequencer: CoinbaseOrderBookL2Sequencer {
                    updates_processed: 0,
                    last_updated_at: chrono::DateTime::from_str("2022-08-04T15:25:05.010758Z").unwrap(),
                },
                input: CoinbaseOrderBookL2Update {
                    product_id: "BTC-USD".to_string(),
                    time_exchange: chrono::DateTime::from_str("2022-08-04T15:25:05.010758Z").unwrap(),
                    changes: vec![],
                },
                expected: Ok(None),
            },
        ];

        for (index, mut test) in tests.into_iter().enumerate() {
            let actual = test.sequencer.validate_sequence(test.input);
            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }

    #[test]
    fn test_coinbase_sequencer_validate_next_update() {
        struct TestCase {
            sequencer: CoinbaseOrderBookL2Sequencer,
            input: CoinbaseOrderBookL2Update,
            expected: Result<Option<CoinbaseOrderBookL2Update>, DataError>,
        }

        let tests = vec![
            TestCase {
                // TC0: valid next update
                sequencer: CoinbaseOrderBookL2Sequencer {
                    updates_processed: 1,
                    last_updated_at: chrono::DateTime::from_str("2022-08-04T15:25:05.010758Z").unwrap(),
                },
                input: CoinbaseOrderBookL2Update {
                    product_id: "BTC-USD".to_string(),
                    time_exchange: chrono::DateTime::from_str("2022-08-04T15:25:06.010758Z").unwrap(),
                    changes: vec![],
                },
                expected: Ok(Some(CoinbaseOrderBookL2Update {
                    product_id: "BTC-USD".to_string(),
                    time_exchange: chrono::DateTime::from_str("2022-08-04T15:25:06.010758Z").unwrap(),
                    changes: vec![],
                })),
            },
            TestCase {
                // TC1: invalid next update with time_exchange <= last_updated_at
                sequencer: CoinbaseOrderBookL2Sequencer {
                    updates_processed: 1,
                    last_updated_at: chrono::DateTime::from_str("2022-08-04T15:25:05.010758Z").unwrap(),
                },
                input: CoinbaseOrderBookL2Update {
                    product_id: "BTC-USD".to_string(),
                    time_exchange: chrono::DateTime::from_str("2022-08-04T15:25:05.010000Z").unwrap(),
                    changes: vec![],
                },
                expected: Ok(None),
            },
        ];

        for (index, mut test) in tests.into_iter().enumerate() {
            let actual = test.sequencer.validate_sequence(test.input);
            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }

    #[test]
    fn test_update_barter_order_book_with_sequenced_updates() {
        struct TestCase {
            sequencer: CoinbaseOrderBookL2Sequencer,
            book: OrderBook,
            input_update: CoinbaseOrderBookL2Update,
            expected: OrderBook,
        }

        let tests = vec![
            TestCase {
                // TC0: Drop any event where time_exchange <= last_updated_at
                sequencer: CoinbaseOrderBookL2Sequencer {
                    updates_processed: 1,
                    last_updated_at: chrono::DateTime::from_str("2022-08-04T15:25:05.010758Z").unwrap(),
                },
                book: OrderBook::new(0, None, vec![Level::new(50, 1)], vec![Level::new(100, 1)]),
                input_update: CoinbaseOrderBookL2Update {
                    product_id: "BTC-USD".to_string(),
                    time_exchange: chrono::DateTime::from_str("2022-08-04T15:25:05.010758Z").unwrap(),
                    changes: vec![],
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
                sequencer: CoinbaseOrderBookL2Sequencer {
                    updates_processed: 1,
                    last_updated_at: chrono::DateTime::from_str("2022-08-04T15:25:05.010758Z").unwrap(),
                },
                book: OrderBook::new(
                    0,
                    None,
                    vec![Level::new(80, 1), Level::new(100, 1), Level::new(90, 1)],
                    vec![Level::new(150, 1), Level::new(110, 1), Level::new(120, 1)],
                ),
                input_update: CoinbaseOrderBookL2Update {
                    product_id: "BTC-USD".to_string(),
                    time_exchange: chrono::DateTime::from_str("2022-08-04T15:25:06.010758Z").unwrap(),
                    changes: vec![
                        // Level exists & new value is 0 => remove Level
                        CoinbaseOrderBookL2Change {
                            side: Side::Buy,
                            price: Decimal::from_str("80").unwrap(),
                            size: Decimal::from_str("0").unwrap(),
                        },
                        // Level exists & new value is > 0 => replace Level
                        CoinbaseOrderBookL2Change {
                            side: Side::Buy,
                            price: Decimal::from_str("90").unwrap(),
                            size: Decimal::from_str("10").unwrap(),
                        },
                        // Level does not exist & new value > 0 => insert new Level
                        CoinbaseOrderBookL2Change {
                            side: Side::Sell,
                            price: Decimal::from_str("200").unwrap(),
                            size: Decimal::from_str("1").unwrap(),
                        },
                    ],
                },
                expected: OrderBook::new(
                    1,
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
            if let Some(valid_update) = test.sequencer.validate_sequence(test.input_update).unwrap() {
                let (bids, asks): (Vec<_>, Vec<_>) = valid_update.changes.into_iter().partition(|change| change.side == Side::Buy);

                let barter_update = OrderBookEvent::Update(OrderBook::new(
                    1,
                    None,
                    bids,
                    asks,
                ));

                test.book.update(barter_update);
            }

            assert_eq!(test.book, test.expected, "TC{index} failed");
        }
    }
}