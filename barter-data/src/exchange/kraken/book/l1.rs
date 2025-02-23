use super::super::KrakenMessage;
use crate::{
    Identifier,
    books::Level,
    event::{MarketEvent, MarketIter},
    exchange::{kraken::channel::KrakenChannel, subscription::ExchangeSub},
    subscription::book::OrderBookL1,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{de::extract_next, subscription::SubscriptionId};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Terse type alias for an [`Kraken`](super::super::Kraken) real-time OrderBook Level1
/// (top of books) WebSocket message.
pub type KrakenOrderBookL1 = KrakenMessage<KrakenOrderBookL1Inner>;

/// [`Kraken`](super::super::Kraken) real-time OrderBook Level1 (top of books) data and the
/// associated [`SubscriptionId`].
///
/// See [`KrakenMessage`] for full raw payload examples.
///
/// See docs: <https://docs.kraken.com/websockets/#message-spread>
#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenOrderBookL1Inner {
    pub subscription_id: SubscriptionId,
    pub spread: KrakenSpread,
}

/// [`Kraken`](super::super::Kraken) best bid and ask.
///
/// See [`KrakenMessage`] for full raw payload examples.
///
/// See docs: <https://docs.kraken.com/websockets/#message-spread>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct KrakenSpread {
    #[serde(with = "rust_decimal::serde::str")]
    pub best_bid_price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub best_ask_price: Decimal,
    #[serde(deserialize_with = "barter_integration::de::de_str_f64_epoch_s_as_datetime_utc")]
    pub time: DateTime<Utc>,
    #[serde(with = "rust_decimal::serde::str")]
    pub best_bid_amount: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub best_ask_amount: Decimal,
}

impl Identifier<Option<SubscriptionId>> for KrakenOrderBookL1Inner {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, KrakenOrderBookL1)>
    for MarketIter<InstrumentKey, OrderBookL1>
{
    fn from(
        (exchange_id, instrument, book): (ExchangeId, InstrumentKey, KrakenOrderBookL1),
    ) -> Self {
        match book {
            KrakenOrderBookL1::Data(book) => {
                let best_ask = if book.spread.best_ask_price.is_zero() {
                    None
                } else {
                    Some(Level::new(
                        book.spread.best_ask_price,
                        book.spread.best_ask_amount,
                    ))
                };

                let best_bid = if book.spread.best_bid_price.is_zero() {
                    None
                } else {
                    Some(Level::new(
                        book.spread.best_bid_price,
                        book.spread.best_bid_amount,
                    ))
                };

                Self(vec![Ok(MarketEvent {
                    time_exchange: book.spread.time,
                    time_received: Utc::now(),
                    exchange: exchange_id,
                    instrument,
                    kind: OrderBookL1 {
                        last_update_time: book.spread.time,
                        best_bid,
                        best_ask,
                    },
                })])
            }
            KrakenOrderBookL1::Event(_) => MarketIter(vec![]),
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for KrakenOrderBookL1Inner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = KrakenOrderBookL1Inner;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenOrderBookL1Inner struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // KrakenOrderBookL1Inner Sequence Format:
                // [channelID, [bid, ask, timestamp, bidVolume, askVolume], channelName, pair]
                // <https://docs.kraken.com/websockets/#message-book>

                // Extract deprecated channelID & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelID")?;

                // Extract spread
                let spread = extract_next(&mut seq, "spread")?;

                // Extract channelName (eg/ "spread") & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelName")?;

                // Extract pair (eg/ "XBT/USD") & map to SubscriptionId (ie/ "spread|{pair}")
                let subscription_id = extract_next::<SeqAccessor, String>(&mut seq, "pair")
                    .map(|market| ExchangeSub::from((KrakenChannel::ORDER_BOOK_L1, market)).id())?;

                // Ignore any additional elements or SerDe will fail
                //  '--> Exchange may add fields without warning
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(KrakenOrderBookL1Inner {
                    subscription_id,
                    spread,
                })
            }
        }

        // Use Visitor implementation to deserialize the KrakenOrderBookL1Inner
        deserializer.deserialize_seq(SeqVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::{
            de::datetime_utc_from_epoch_duration, error::SocketError, subscription::SubscriptionId,
        };
        use rust_decimal_macros::dec;

        #[test]
        fn test_kraken_message_order_book_l1() {
            struct TestCase {
                input: &'static str,
                expected: Result<KrakenOrderBookL1, SocketError>,
            }

            let tests = vec![TestCase {
                // TC0: valid KrakenOrderBookL1::Data(KrakenOrderBookL1Inner)
                input: r#"
                    [
                        0,
                        [
                            "5698.40000",
                            "5700.00000",
                            "1542057299.545897",
                            "1.01234567",
                            "0.98765432"
                        ],
                        "spread",
                        "XBT/USD"
                    ]
                    "#,
                expected: Ok(KrakenOrderBookL1::Data(KrakenOrderBookL1Inner {
                    subscription_id: SubscriptionId::from("spread|XBT/USD"),
                    spread: KrakenSpread {
                        best_bid_price: dec!(5698.40000),
                        best_bid_amount: dec!(1.01234567),
                        time: datetime_utc_from_epoch_duration(std::time::Duration::from_secs_f64(
                            1542057299.545897,
                        )),
                        best_ask_price: dec!(5700.00000),
                        best_ask_amount: dec!(0.98765432),
                    },
                })),
            }];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<KrakenOrderBookL1>(test.input);
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
    }
}
