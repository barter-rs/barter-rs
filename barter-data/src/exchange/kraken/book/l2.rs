use super::super::{Kraken, channel::KrakenChannel, message::KrakenMessage};
use crate::{
    Identifier,
    books::{Level, OrderBook},
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::{Connector, subscription::ExchangeSub},
    subscription::{
        Map,
        book::{OrderBookEvent, OrderBooksL2},
    },
    transformer::ExchangeTransformer,
};
use async_trait::async_trait;
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    Transformer,
    protocol::websocket::WsMessage,
    serde::de::{datetime_utc_from_epoch_duration, extract_next},
    subscription::SubscriptionId,
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

/// Number of [`Level`]s on each side of the book the [`Kraken`] CRC32 checksum is calculated
/// over.
///
/// See docs: <https://docs.kraken.com/api/docs/guides/spot-ws-book-v1>
const KRAKEN_CHECKSUM_DEPTH: usize = 10;

/// Terse type alias for a [`Kraken`] real-time OrderBook Level2 WebSocket message.
pub type KrakenOrderBookL2Message = KrakenMessage<KrakenOrderBookL2Inner>;

/// [`Kraken`] real-time OrderBook Level2 data and the associated [`SubscriptionId`].
///
/// ### Raw Payload Examples
/// See docs: <https://docs.kraken.com/api/docs/guides/spot-ws-book-v1>
///
/// #### Snapshot
/// ```json
/// [
///     0,
///     {
///         "as": [["5541.30000", "2.50700000", "1534614248.123678"]],
///         "bs": [["5541.20000", "1.52900000", "1534614248.765567"]]
///     },
///     "book-10",
///     "XBT/USD"
/// ]
/// ```
///
/// #### Update
/// Bid and ask updates may arrive in a single payload object, or as two separate payload
/// objects within the same message. The CRC32 checksum of the top ten levels is included with
/// the final payload object.
/// ```json
/// [
///     1234,
///     { "a": [["5541.30000", "2.50700000", "1534614248.456738"]] },
///     { "b": [["5541.30000", "0.00000000", "1534614335.345903"]], "c": "974942666" },
///     "book-10",
///     "XBT/USD"
/// ]
/// ```
#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct KrakenOrderBookL2Inner {
    pub subscription_id: SubscriptionId,
    pub kind: KrakenOrderBookL2Kind,
}

/// [`Kraken`] OrderBook Level2 message payload variants.
#[derive(Clone, PartialEq, Debug, Serialize)]
pub enum KrakenOrderBookL2Kind {
    Snapshot {
        bids: Vec<KrakenLevel>,
        asks: Vec<KrakenLevel>,
    },
    Update {
        bids: Vec<KrakenLevel>,
        asks: Vec<KrakenLevel>,
        checksum: Option<u32>,
    },
}

/// [`Kraken`] OrderBook [`Level`] with the original wire strings retained.
///
/// The CRC32 checksum is defined over the exact price and volume strings sent by the exchange,
/// so they must be preserved alongside the parsed [`Decimal`] values.
#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct KrakenLevel {
    pub price: Decimal,
    pub amount: Decimal,
    pub time: DateTime<Utc>,
    pub price_raw: String,
    pub amount_raw: String,
}

impl From<&KrakenLevel> for Level {
    fn from(level: &KrakenLevel) -> Self {
        Self {
            price: level.price,
            amount: level.amount,
        }
    }
}

impl<'de> Deserialize<'de> for KrakenLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = KrakenLevel;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenLevel array from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // KrakenLevel Sequence Format:
                // [price, volume, timestamp] or [price, volume, timestamp, "r"] (republished)
                let price_raw = extract_next::<SeqAccessor, String>(&mut seq, "price")?;
                let amount_raw = extract_next::<SeqAccessor, String>(&mut seq, "volume")?;
                let time_raw = extract_next::<SeqAccessor, String>(&mut seq, "timestamp")?;

                // Ignore any additional elements (eg/ "r" republish flag)
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                let price = price_raw
                    .parse::<Decimal>()
                    .map_err(serde::de::Error::custom)?;
                let amount = amount_raw
                    .parse::<Decimal>()
                    .map_err(serde::de::Error::custom)?;
                let time_secs = time_raw.parse::<f64>().map_err(serde::de::Error::custom)?;
                let time =
                    datetime_utc_from_epoch_duration(std::time::Duration::from_secs_f64(time_secs));

                Ok(KrakenLevel {
                    price,
                    amount,
                    time,
                    price_raw,
                    amount_raw,
                })
            }
        }

        deserializer.deserialize_seq(SeqVisitor)
    }
}

/// Intermediate representation of one payload object within a [`Kraken`] book message.
///
/// A single message can contain up to two payload objects (one for asks, one for bids), with
/// the checksum attached to the final object.
#[derive(Default, Deserialize)]
struct KrakenBookPayload {
    #[serde(rename = "as", default)]
    asks_snapshot: Option<Vec<KrakenLevel>>,
    #[serde(rename = "bs", default)]
    bids_snapshot: Option<Vec<KrakenLevel>>,
    #[serde(rename = "a", default)]
    asks: Option<Vec<KrakenLevel>>,
    #[serde(rename = "b", default)]
    bids: Option<Vec<KrakenLevel>>,
    #[serde(rename = "c", default)]
    checksum: Option<String>,
}

/// One positional element of the [`Kraken`] book message array: either a payload object or a
/// trailing metadata string (channelName, then pair).
#[derive(Deserialize)]
#[serde(untagged)]
enum KrakenBookElement {
    Payload(KrakenBookPayload),
    Text(serde::de::IgnoredAny),
}

impl<'de> Deserialize<'de> for KrakenOrderBookL2Inner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = KrakenOrderBookL2Inner;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenOrderBookL2Inner struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // KrakenOrderBookL2Inner Sequence Format:
                // [channelID, payload, (payload,) channelName, pair]

                // Extract deprecated channelID & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelID")?;

                // Extract one or two payload objects, then the channelName string
                let mut asks_snapshot = None;
                let mut bids_snapshot = None;
                let mut asks = None;
                let mut bids = None;
                let mut checksum = None;

                // Iterate payload objects until the channelName (eg/ "book-10") is
                // reached & ignored
                while let KrakenBookElement::Payload(payload) =
                    extract_next::<SeqAccessor, KrakenBookElement>(
                        &mut seq,
                        "payload | channelName",
                    )?
                {
                    if payload.asks_snapshot.is_some() {
                        asks_snapshot = payload.asks_snapshot;
                    }
                    if payload.bids_snapshot.is_some() {
                        bids_snapshot = payload.bids_snapshot;
                    }
                    if payload.asks.is_some() {
                        asks = payload.asks;
                    }
                    if payload.bids.is_some() {
                        bids = payload.bids;
                    }
                    if payload.checksum.is_some() {
                        checksum = payload.checksum;
                    }
                }

                // Extract pair (eg/ "XBT/USD") & map to SubscriptionId (ie/ "book|{pair}")
                let subscription_id = extract_next::<SeqAccessor, String>(&mut seq, "pair")
                    .map(|market| ExchangeSub::from((KrakenChannel::ORDER_BOOK_L2, market)).id())?;

                // Ignore any additional elements or SerDe will fail
                //  '--> Exchange may add fields without warning
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                let kind = if asks_snapshot.is_some() || bids_snapshot.is_some() {
                    KrakenOrderBookL2Kind::Snapshot {
                        bids: bids_snapshot.unwrap_or_default(),
                        asks: asks_snapshot.unwrap_or_default(),
                    }
                } else {
                    let checksum = checksum
                        .map(|value| value.parse::<u32>())
                        .transpose()
                        .map_err(serde::de::Error::custom)?;

                    KrakenOrderBookL2Kind::Update {
                        bids: bids.unwrap_or_default(),
                        asks: asks.unwrap_or_default(),
                        checksum,
                    }
                };

                Ok(KrakenOrderBookL2Inner {
                    subscription_id,
                    kind,
                })
            }
        }

        deserializer.deserialize_seq(SeqVisitor)
    }
}

impl Identifier<Option<SubscriptionId>> for KrakenOrderBookL2Inner {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

/// [`Kraken`] raw level data associated with a price in the [`KrakenLocalBook`].
#[derive(Clone, Debug)]
struct KrakenRawLevel {
    time: DateTime<Utc>,
    price_raw: String,
    amount_raw: String,
}

/// Local replica of a [`Kraken`] OrderBook, required to validate the CRC32 checksum included
/// with each update against the current top ten levels of each side.
#[derive(Debug, Default)]
struct KrakenLocalBook {
    bids: BTreeMap<Decimal, KrakenRawLevel>,
    asks: BTreeMap<Decimal, KrakenRawLevel>,
    sequence: u64,
}

impl KrakenLocalBook {
    fn from_snapshot(bids: &[KrakenLevel], asks: &[KrakenLevel]) -> Self {
        let mut book = Self::default();
        Self::upsert(&mut book.bids, bids);
        Self::upsert(&mut book.asks, asks);
        book
    }

    fn apply_update(&mut self, bids: &[KrakenLevel], asks: &[KrakenLevel]) {
        Self::upsert(&mut self.bids, bids);
        Self::upsert(&mut self.asks, asks);
        self.sequence += 1;
    }

    fn upsert(side: &mut BTreeMap<Decimal, KrakenRawLevel>, levels: &[KrakenLevel]) {
        for level in levels {
            if level.amount.is_zero() {
                side.remove(&level.price);
            } else {
                side.insert(
                    level.price,
                    KrakenRawLevel {
                        time: level.time,
                        price_raw: level.price_raw.clone(),
                        amount_raw: level.amount_raw.clone(),
                    },
                );
            }
        }
    }

    /// Calculate the CRC32 checksum of the top ten [`Level`]s on each side of the local book.
    ///
    /// The checksum input is built from the top ten asks (ascending price) followed by the top
    /// ten bids (descending price). For each level the price and volume wire strings are
    /// concatenated with the decimal point removed and leading zeros trimmed.
    ///
    /// See docs: <https://docs.kraken.com/api/docs/guides/spot-ws-book-v1>
    fn checksum(&self) -> u32 {
        let mut input = String::new();

        for level in self.asks.values().take(KRAKEN_CHECKSUM_DEPTH) {
            input.push_str(&trim_checksum_field(&level.price_raw));
            input.push_str(&trim_checksum_field(&level.amount_raw));
        }

        for (price_raw, amount_raw) in self
            .bids
            .values()
            .rev()
            .take(KRAKEN_CHECKSUM_DEPTH)
            .map(|level| (&level.price_raw, &level.amount_raw))
        {
            input.push_str(&trim_checksum_field(price_raw));
            input.push_str(&trim_checksum_field(amount_raw));
        }

        crc32fast::hash(input.as_bytes())
    }

    /// Most recent exchange time across both sides of the local book.
    fn time_engine(&self) -> Option<DateTime<Utc>> {
        let best_bid_time = self.bids.values().next_back().map(|level| level.time);
        let best_ask_time = self.asks.values().next().map(|level| level.time);
        best_bid_time.max(best_ask_time)
    }
}

/// Remove the decimal point and trim leading zeros from a [`Kraken`] price or volume string,
/// as defined by the checksum algorithm.
///
/// eg/ "0.00305000" -> "000305000" -> "305000"
fn trim_checksum_field(value: &str) -> String {
    let without_point: String = value.chars().filter(|char| *char != '.').collect();
    without_point.trim_start_matches('0').to_string()
}

/// [`Kraken`] instrument metadata associated with an active OrderBook Level2 subscription.
#[derive(Debug, Constructor)]
pub struct KrakenOrderBookL2Meta<InstrumentKey> {
    pub key: InstrumentKey,
    book: Option<KrakenLocalBook>,
}

/// [`ExchangeTransformer`] implementation for [`Kraken`] OrderBook Level2 streams.
///
/// Maintains a local replica of every subscribed book in order to validate the CRC32 checksum
/// included with each update. A checksum mismatch produces a terminal
/// [`DataError::InvalidChecksum`], causing the stream to re-initialise with a fresh snapshot.
#[derive(Debug)]
pub struct KrakenOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<KrakenOrderBookL2Meta<InstrumentKey>>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<Kraken, InstrumentKey, OrderBooksL2>
    for KrakenOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync,
{
    async fn init(
        instrument_map: Map<InstrumentKey>,
        _: &[MarketEvent<InstrumentKey, OrderBookEvent>],
        _: UnboundedSender<WsMessage>,
    ) -> Result<Self, DataError> {
        let instrument_map = instrument_map
            .0
            .into_iter()
            .map(|(sub_id, instrument_key)| {
                (sub_id, KrakenOrderBookL2Meta::new(instrument_key, None))
            })
            .collect();

        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for KrakenOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = KrakenOrderBookL2Message;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        let inner = match input {
            KrakenMessage::Data(inner) => inner,
            KrakenMessage::Event(_) => return vec![],
        };

        // Find Instrument associated with Input and transform
        let instrument = match self.instrument_map.find_mut(&subscription_id) {
            Ok(instrument) => instrument,
            Err(unidentifiable) => return vec![Err(DataError::from(unidentifiable))],
        };

        match inner.kind {
            KrakenOrderBookL2Kind::Snapshot { bids, asks } => {
                let book = KrakenLocalBook::from_snapshot(&bids, &asks);
                let time_engine = book.time_engine();
                let sequence = book.sequence;
                instrument.book = Some(book);

                MarketIter::<InstrumentKey, OrderBookEvent>::from((
                    Kraken::ID,
                    instrument.key.clone(),
                    KrakenBookEvent {
                        time_engine,
                        event: OrderBookEvent::Snapshot(OrderBook::new(
                            sequence,
                            time_engine,
                            bids.iter().map(Level::from),
                            asks.iter().map(Level::from),
                        )),
                    },
                ))
                .0
            }
            KrakenOrderBookL2Kind::Update {
                bids,
                asks,
                checksum,
            } => {
                // Could happen if we receive an update message before the snapshot
                let Some(book) = &mut instrument.book else {
                    debug!("Update message received before initial Snapshot");
                    return vec![];
                };

                book.apply_update(&bids, &asks);

                if let Some(expected) = checksum {
                    let computed = book.checksum();
                    if computed != expected {
                        return vec![Err(DataError::InvalidChecksum { expected, computed })];
                    }
                }

                let time_engine = book.time_engine();
                let sequence = book.sequence;

                MarketIter::<InstrumentKey, OrderBookEvent>::from((
                    Kraken::ID,
                    instrument.key.clone(),
                    KrakenBookEvent {
                        time_engine,
                        event: OrderBookEvent::Update(OrderBook::new(
                            sequence,
                            time_engine,
                            bids.iter().map(Level::from),
                            asks.iter().map(Level::from),
                        )),
                    },
                ))
                .0
            }
        }
    }
}

/// Convenience container pairing an [`OrderBookEvent`] with its exchange time for
/// [`MarketIter`] construction.
struct KrakenBookEvent {
    time_engine: Option<DateTime<Utc>>,
    event: OrderBookEvent,
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, KrakenBookEvent)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from((exchange, instrument, message): (ExchangeId, InstrumentKey, KrakenBookEvent)) -> Self {
        let time_received = Utc::now();
        Self(vec![Ok(MarketEvent {
            time_exchange: message.time_engine.unwrap_or(time_received),
            time_received,
            exchange,
            instrument,
            kind: message.event,
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::serde::de::datetime_utc_from_epoch_duration;
        use rust_decimal_macros::dec;
        use std::time::Duration;

        #[test]
        fn test_kraken_level() {
            struct TestCase {
                input: &'static str,
                expected: KrakenLevel,
            }

            let tests = vec![
                TestCase {
                    // TC0: valid three element level
                    input: r#"["5541.30000", "2.50700000", "1534614248.123678"]"#,
                    expected: KrakenLevel {
                        price: dec!(5541.30000),
                        amount: dec!(2.50700000),
                        time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                            1534614248.123678,
                        )),
                        price_raw: "5541.30000".to_string(),
                        amount_raw: "2.50700000".to_string(),
                    },
                },
                TestCase {
                    // TC1: valid four element level with republish flag
                    input: r#"["5541.30000", "2.50700000", "1534614248.123678", "r"]"#,
                    expected: KrakenLevel {
                        price: dec!(5541.30000),
                        amount: dec!(2.50700000),
                        time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(
                            1534614248.123678,
                        )),
                        price_raw: "5541.30000".to_string(),
                        amount_raw: "2.50700000".to_string(),
                    },
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<KrakenLevel>(test.input).unwrap();
                assert_eq!(actual, test.expected, "TC{index} failed");
            }
        }

        #[test]
        fn test_kraken_order_book_l2_snapshot() {
            let input = r#"
                [
                    0,
                    {
                        "as": [["5541.30000", "2.50700000", "1534614248.123678"]],
                        "bs": [["5541.20000", "1.52900000", "1534614248.765567"]]
                    },
                    "book-10",
                    "XBT/USD"
                ]
                "#;

            let actual = serde_json::from_str::<KrakenOrderBookL2Message>(input).unwrap();

            let KrakenMessage::Data(inner) = actual else {
                panic!("expected KrakenMessage::Data");
            };
            assert_eq!(inner.subscription_id, SubscriptionId::from("book|XBT/USD"));

            let KrakenOrderBookL2Kind::Snapshot { bids, asks } = inner.kind else {
                panic!("expected KrakenOrderBookL2Kind::Snapshot");
            };
            assert_eq!(asks.len(), 1);
            assert_eq!(asks[0].price, dec!(5541.30000));
            assert_eq!(bids.len(), 1);
            assert_eq!(bids[0].price, dec!(5541.20000));
        }

        #[test]
        fn test_kraken_order_book_l2_update_with_split_payload() {
            let input = r#"
                [
                    1234,
                    { "a": [["5541.30000", "2.50700000", "1534614248.456738"]] },
                    { "b": [["5541.30000", "0.00000000", "1534614335.345903"]], "c": "974942666" },
                    "book-10",
                    "XBT/USD"
                ]
                "#;

            let actual = serde_json::from_str::<KrakenOrderBookL2Message>(input).unwrap();

            let KrakenMessage::Data(inner) = actual else {
                panic!("expected KrakenMessage::Data");
            };

            let KrakenOrderBookL2Kind::Update {
                bids,
                asks,
                checksum,
            } = inner.kind
            else {
                panic!("expected KrakenOrderBookL2Kind::Update");
            };
            assert_eq!(asks.len(), 1);
            assert_eq!(bids.len(), 1);
            assert!(bids[0].amount.is_zero());
            assert_eq!(checksum, Some(974942666));
        }

        #[test]
        fn test_kraken_order_book_l2_update_single_payload_no_checksum() {
            let input = r#"
                [
                    1234,
                    { "b": [["5541.20000", "1.00000000", "1534614335.345903"]] },
                    "book-10",
                    "XBT/USD"
                ]
                "#;

            let actual = serde_json::from_str::<KrakenOrderBookL2Message>(input).unwrap();

            let KrakenMessage::Data(inner) = actual else {
                panic!("expected KrakenMessage::Data");
            };
            let KrakenOrderBookL2Kind::Update {
                bids,
                asks,
                checksum,
            } = inner.kind
            else {
                panic!("expected KrakenOrderBookL2Kind::Update");
            };
            assert_eq!(bids.len(), 1);
            assert!(asks.is_empty());
            assert_eq!(checksum, None);
        }

        #[test]
        fn test_kraken_order_book_l2_heartbeat() {
            let input = r#"{"event": "heartbeat"}"#;
            let actual = serde_json::from_str::<KrakenOrderBookL2Message>(input).unwrap();
            assert!(matches!(actual, KrakenMessage::Event(_)));
        }
    }

    mod book {
        use super::*;
        use barter_integration::serde::de::datetime_utc_from_epoch_duration;
        use rust_decimal_macros::dec;
        use std::time::Duration;

        fn level(price: &str, amount: &str, time_secs: f64) -> KrakenLevel {
            KrakenLevel {
                price: price.parse().unwrap(),
                amount: amount.parse().unwrap(),
                time: datetime_utc_from_epoch_duration(Duration::from_secs_f64(time_secs)),
                price_raw: price.to_string(),
                amount_raw: amount.to_string(),
            }
        }

        #[test]
        fn test_trim_checksum_field() {
            struct TestCase {
                input: &'static str,
                expected: &'static str,
            }

            let tests = vec![
                TestCase {
                    input: "0.00305000",
                    expected: "305000",
                },
                TestCase {
                    input: "5541.30000",
                    expected: "554130000",
                },
                TestCase {
                    input: "34.10000000",
                    expected: "3410000000",
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(
                    trim_checksum_field(test.input),
                    test.expected,
                    "TC{index} failed"
                );
            }
        }

        #[test]
        fn test_local_book_checksum_ordering() {
            // Checksum input must be the top ten asks in ascending price order, followed by
            // the top ten bids in descending price order, with each price and volume string
            // stripped of its decimal point and leading zeros
            let book = KrakenLocalBook::from_snapshot(
                &[
                    level("0.85000", "10.00000000", 1.0),
                    level("0.84000", "5.00000000", 1.0),
                ],
                &[
                    level("0.87000", "2.00000000", 1.0),
                    level("0.86000", "1.00000000", 1.0),
                ],
            );

            let expected_input = concat!(
                // Asks ascending: 0.86000, 0.87000
                "86000",
                "100000000",
                "87000",
                "200000000",
                // Bids descending: 0.85000, 0.84000
                "85000",
                "1000000000",
                "84000",
                "500000000",
            );

            assert_eq!(book.checksum(), crc32fast::hash(expected_input.as_bytes()));
        }

        #[test]
        fn test_local_book_update_upserts_and_removes() {
            let mut book = KrakenLocalBook::from_snapshot(
                &[level("100.0", "1.0", 1.0)],
                &[level("101.0", "1.0", 1.0)],
            );

            // Zero volume removes the level; non-zero volume upserts
            book.apply_update(
                &[level("100.0", "0.0", 2.0), level("99.0", "5.0", 2.0)],
                &[level("101.0", "3.0", 2.0)],
            );

            assert_eq!(book.sequence, 1);
            assert_eq!(book.bids.len(), 1);
            assert!(book.bids.contains_key(&dec!(99.0)));
            assert_eq!(book.asks.len(), 1);
            assert_eq!(book.asks[&dec!(101.0)].amount_raw, "3.0");
        }
    }
}
