use crate::{
    Identifier,
    books::{Level, OrderBook},
    exchange::kraken::message::KrakenMessage,
    subscription::book::OrderBookEvent,
    event::{MarketEvent, MarketIter},
};
use barter_integration::{
    de::extract_next,
    subscription::SubscriptionId,
};
use barter_instrument::exchange::ExchangeId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Terse type alias for an [`Kraken`](crate::exchange::kraken::KrakenSpot) real-time OrderBook Level2
/// (full depth) WebSocket message.
pub type KrakenOrderBookL2 = KrakenMessage<KrakenOrderBookL2Inner>;

/// [`Kraken`](crate::exchange::kraken::KrakenSpot) L2 OrderBook data and the
/// associated [`SubscriptionId`].
///
/// See docs: <https://docs.kraken.com/websockets/#message-book>
#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct KrakenOrderBookL2Inner {
    pub subscription_id: SubscriptionId,
    pub data: KrakenOrderBookL2Data,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum KrakenOrderBookL2Data {
    Snapshot(KrakenBookSnapshot),
    Update(KrakenBookUpdate),
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct KrakenBookSnapshot {
    #[serde(alias = "as")]
    pub asks: Vec<KrakenBookLevel>,
    #[serde(alias = "bs")]
    pub bids: Vec<KrakenBookLevel>,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct KrakenBookUpdate {
    #[serde(default, rename = "a")]
    pub asks: Vec<KrakenBookLevel>,
    #[serde(default, rename = "b")]
    pub bids: Vec<KrakenBookLevel>,
    #[serde(rename = "c")]
    pub checksum: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
pub struct KrakenBookLevel {
    pub price: Decimal,
    pub amount: Decimal,
}

impl From<KrakenBookLevel> for Level {
    fn from(level: KrakenBookLevel) -> Self {
        Self {
            price: level.price,
            amount: level.amount,
        }
    }
}

impl Identifier<Option<SubscriptionId>> for KrakenOrderBookL2Inner {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
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
                // Kraken OrderBook L2 Format:
                // [channelID, data, channelName, pair]
                // <https://docs.kraken.com/websockets/#message-book>

                // Extract deprecated channelID & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelID")?;

                // Extract Data (Snapshot or Update)
                let data = extract_next(&mut seq, "KrakenOrderBookL2Data")?;

                // Extract channelName (eg/ "book-100") & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelName")?;

                // Extract pair (eg/ "XBT/USD") & map to SubscriptionId (ie/ "book|{pair}")
                let subscription_id = extract_next::<SeqAccessor, String>(&mut seq, "pair")
                    .map(|pair| SubscriptionId::from(format!("book|{pair}")))?;

                // Ignore any additional elements
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(KrakenOrderBookL2Inner {
                    subscription_id,
                    data,
                })
            }
        }

        deserializer.deserialize_seq(SeqVisitor)
    }
}

impl<'de> Deserialize<'de> for KrakenBookLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct LevelVisitor;

        impl<'de> serde::de::Visitor<'de> for LevelVisitor {
            type Value = KrakenBookLevel;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenBookLevel array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let price = extract_next::<A, String>(&mut seq, "price")?
                    .parse()
                    .map_err(serde::de::Error::custom)?;
                let amount = extract_next::<A, String>(&mut seq, "amount")?
                    .parse()
                    .map_err(serde::de::Error::custom)?;
                
                // Consume remaining elements
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(KrakenBookLevel { price, amount })
            }
        }
        
        deserializer.deserialize_seq(LevelVisitor)
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, KrakenOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from((exchange, instrument, message): (ExchangeId, InstrumentKey, KrakenOrderBookL2)) -> Self {
        let KrakenOrderBookL2Inner { data, .. } = match message {
             KrakenMessage::Data(data) => data,
             KrakenMessage::Event(_) => return Self(vec![]),
        };

        let event = match data {
            KrakenOrderBookL2Data::Snapshot(snap) => {
                OrderBookEvent::Snapshot(OrderBook::new(
                     0,
                     Some(Utc::now()),
                     snap.bids.into_iter().map(Level::from),
                     snap.asks.into_iter().map(Level::from),
                ))
            },
            KrakenOrderBookL2Data::Update(update) => {
                OrderBookEvent::Update(OrderBook::new(
                     0,
                     Some(Utc::now()),
                     update.bids.into_iter().map(Level::from),
                     update.asks.into_iter().map(Level::from),
                ))
            }
        };

        Self(vec![Ok(MarketEvent {
            exchange,
            instrument,
            kind: event,
            time_exchange: Utc::now(),
            time_received: Utc::now(),
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    mod de {
        use super::*;
        use crate::exchange::kraken::message::KrakenMessage;
        use barter_integration::{error::SocketError, subscription::SubscriptionId};

        #[test]
        fn test_kraken_order_book_l2_snapshot() {
            struct TestCase {
                input: &'static str,
                expected: Result<KrakenOrderBookL2, SocketError>,
            }

            let tests = vec![TestCase {
                // TC0: valid KrakenOrderBookL2 snapshot
                input: r#"
                    [
                        0,
                        {
                            "as": [
                                ["5541.30000", "2.50700000", "1534614248.123456"],
                                ["5541.80000", "0.33000000", "1534614248.123457"]
                            ],
                            "bs": [
                                ["5541.20000", "1.52900000", "1534614248.123458"],
                                ["5541.10000", "0.50000000", "1534614248.123459"]
                            ]
                        },
                        "book-100",
                        "XBT/USD"
                    ]
                "#,
                expected: Ok(KrakenMessage::Data(KrakenOrderBookL2Inner {
                    subscription_id: SubscriptionId::from("book|XBT/USD"),
                    data: KrakenOrderBookL2Data::Snapshot(KrakenBookSnapshot {
                        asks: vec![
                            KrakenBookLevel { price: dec!(5541.30000), amount: dec!(2.50700000) },
                            KrakenBookLevel { price: dec!(5541.80000), amount: dec!(0.33000000) },
                        ],
                        bids: vec![
                            KrakenBookLevel { price: dec!(5541.20000), amount: dec!(1.52900000) },
                            KrakenBookLevel { price: dec!(5541.10000), amount: dec!(0.50000000) },
                        ],
                    }),
                })),
            }];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<KrakenOrderBookL2>(test.input);
                match (actual, test.expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(actual, expected, "TC{} failed", index)
                    }
                    (Err(_), Err(_)) => {
                        // Test passed
                    }
                    (actual, expected) => {
                        panic!(
                            "TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                        );
                    }
                }
            }
        }

        #[test]
        fn test_kraken_order_book_l2_update() {
            struct TestCase {
                input: &'static str,
                expected: Result<KrakenOrderBookL2, SocketError>,
            }

            let tests = vec![TestCase {
                // TC0: valid KrakenOrderBookL2 update
                input: r#"
                    [
                        0,
                        {
                            "a": [
                                ["5541.30000", "1.00000000", "1534614335.345678"]
                            ],
                            "b": [
                                ["5541.20000", "0.00000000", "1534614335.345679"]
                            ],
                            "c": "974942666"
                        },
                        "book-100",
                        "XBT/USD"
                    ]
                "#,
                expected: Ok(KrakenMessage::Data(KrakenOrderBookL2Inner {
                    subscription_id: SubscriptionId::from("book|XBT/USD"),
                    data: KrakenOrderBookL2Data::Update(KrakenBookUpdate {
                        asks: vec![
                            KrakenBookLevel { price: dec!(5541.30000), amount: dec!(1.00000000) },
                        ],
                        bids: vec![
                            KrakenBookLevel { price: dec!(5541.20000), amount: dec!(0.00000000) },
                        ],
                        checksum: Some("974942666".to_string()),
                    }),
                })),
            }];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<KrakenOrderBookL2>(test.input);
                match (actual, test.expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(actual, expected, "TC{} failed", index)
                    }
                    (Err(_), Err(_)) => {
                        // Test passed
                    }
                    (actual, expected) => {
                        panic!(
                            "TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                        );
                    }
                }
            }
        }

        #[test]
        fn test_kraken_book_level_from_conversion() {
            let kraken_level = KrakenBookLevel {
                price: dec!(5541.20000),
                amount: dec!(1.52900000),
            };
            let level: Level = kraken_level.into();
            assert_eq!(level.price, dec!(5541.20000));
            assert_eq!(level.amount, dec!(1.52900000));
        }

        #[test]
        fn test_kraken_book_level_zero_amount() {
            // Zero amount indicates removal of the price level
            let kraken_level = KrakenBookLevel {
                price: dec!(5541.20000),
                amount: dec!(0),
            };
            let level: Level = kraken_level.into();
            assert_eq!(level.amount, dec!(0));
        }
    }
}
