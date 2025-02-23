use super::KrakenMessage;
use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
    subscription::trade::PublicTrade,
};
use barter_instrument::{Side, exchange::ExchangeId};
use barter_integration::{
    de::{datetime_utc_from_epoch_duration, extract_next},
    subscription::SubscriptionId,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Terse type alias for an [`Kraken`](super::Kraken) real-time trades WebSocket message.
pub type KrakenTrades = KrakenMessage<KrakenTradesInner>;

/// Collection of [`KrakenTrade`] items with an associated [`SubscriptionId`] (eg/ "trade|XBT/USD").
///
/// See [`KrakenMessage`] for full raw payload examples.
///
/// See docs: <https://docs.kraken.com/websockets/#message-trade>
#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenTradesInner {
    pub subscription_id: SubscriptionId,
    pub trades: Vec<KrakenTrade>,
}

/// [`Kraken`](super::Kraken) trade.
///
/// See [`KrakenMessage`] for full raw payload examples.
///
/// See docs: <https://docs.kraken.com/websockets/#message-trade>
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Serialize)]
pub struct KrakenTrade {
    pub price: f64,
    #[serde(rename = "quantity")]
    pub amount: f64,
    pub time: DateTime<Utc>,
    pub side: Side,
}

impl Identifier<Option<SubscriptionId>> for KrakenTradesInner {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

/// Generate a custom [`Kraken`](super::Kraken) trade identifier since it is not provided in the
/// [`KrakenTrade`] model.
fn custom_kraken_trade_id(trade: &KrakenTrade) -> String {
    format!(
        "{}_{}_{}_{}",
        trade.time.timestamp_micros(),
        trade.side,
        trade.price,
        trade.amount
    )
}

impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, KrakenTrades)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange, instrument, trades): (ExchangeId, InstrumentKey, KrakenTrades)) -> Self {
        match trades {
            KrakenTrades::Data(trades) => trades
                .trades
                .into_iter()
                .map(|trade| {
                    Ok(MarketEvent {
                        time_exchange: trade.time,
                        time_received: Utc::now(),
                        exchange,
                        instrument: instrument.clone(),
                        kind: PublicTrade {
                            id: custom_kraken_trade_id(&trade),
                            price: trade.price,
                            amount: trade.amount,
                            side: trade.side,
                        },
                    })
                })
                .collect(),
            KrakenTrades::Event(_) => Self(vec![]),
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for KrakenTradesInner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = KrakenTradesInner;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenTradesInner struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // KrakenTrades Sequence Format:
                // [channelID, [[price, volume, time, side, orderType, misc]], channelName, pair]
                // <https://docs.kraken.com/websockets/#message-trade>

                // Extract deprecated channelID & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelID")?;

                // Extract Vec<KrakenTrade>
                let trades = extract_next(&mut seq, "Vec<KrakenTrade>")?;

                // Extract channelName (eg/ "trade") & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelName")?;

                // Extract pair (eg/ "XBT/USD") & map to SubscriptionId (ie/ "trade|{pair}")
                let subscription_id = extract_next::<SeqAccessor, String>(&mut seq, "pair")
                    .map(|pair| SubscriptionId::from(format!("trade|{pair}")))?;

                // Ignore any additional elements or SerDe will fail
                //  '--> Exchange may add fields without warning
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(KrakenTradesInner {
                    subscription_id,
                    trades,
                })
            }
        }

        // Use Visitor implementation to deserialise the KrakenTrades
        deserializer.deserialize_seq(SeqVisitor)
    }
}

impl<'de> serde::de::Deserialize<'de> for KrakenTrade {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = KrakenTrade;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenTrade struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // KrakenTrade Sequence Format:
                // [price, volume, time, side, orderType, misc]
                // <https://docs.kraken.com/websockets/#message-trade>

                // Extract String price & parse to f64
                let price = extract_next::<SeqAccessor, String>(&mut seq, "price")?
                    .parse()
                    .map_err(serde::de::Error::custom)?;

                // Extract String amount & parse to f64
                let amount = extract_next::<SeqAccessor, String>(&mut seq, "quantity")?
                    .parse()
                    .map_err(serde::de::Error::custom)?;

                // Extract String price, parse to f64, map to DateTime<Utc>
                let time = extract_next::<SeqAccessor, String>(&mut seq, "time")?
                    .parse()
                    .map(|time| {
                        datetime_utc_from_epoch_duration(std::time::Duration::from_secs_f64(time))
                    })
                    .map_err(serde::de::Error::custom)?;

                // Extract Side
                let side: Side = extract_next(&mut seq, "side")?;

                // Ignore any additional elements or SerDe will fail
                //  '--> Exchange may add fields without warning
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(KrakenTrade {
                    price,
                    amount,
                    time,
                    side,
                })
            }
        }

        // Use Visitor implementation to deserialise the KrakenTrade
        deserializer.deserialize_seq(SeqVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_instrument::Side;
        use barter_integration::{
            de::datetime_utc_from_epoch_duration, error::SocketError, subscription::SubscriptionId,
        };

        #[test]
        fn test_kraken_message_trades() {
            struct TestCase {
                input: &'static str,
                expected: Result<KrakenTrades, SocketError>,
            }

            let tests = vec![TestCase {
                // TC0: valid KrakenTrades::Data(KrakenTradesInner)
                input: r#"
                    [
                        0,
                        [
                            [
                                "5541.20000",
                                "0.15850568",
                                "1534614057.321597",
                                "s",
                                "l",
                                ""
                            ],
                            [
                                "6060.00000",
                                "0.02455000",
                                "1534614057.324998",
                                "b",
                                "l",
                                ""
                            ]
                        ],
                      "trade",
                      "XBT/USD"
                    ]
                    "#,
                expected: Ok(KrakenTrades::Data(KrakenTradesInner {
                    subscription_id: SubscriptionId::from("trade|XBT/USD"),
                    trades: vec![
                        KrakenTrade {
                            price: 5541.2,
                            amount: 0.15850568,
                            time: datetime_utc_from_epoch_duration(
                                std::time::Duration::from_secs_f64(1534614057.321597),
                            ),
                            side: Side::Sell,
                        },
                        KrakenTrade {
                            price: 6060.0,
                            amount: 0.02455000,
                            time: datetime_utc_from_epoch_duration(
                                std::time::Duration::from_secs_f64(1534614057.324998),
                            ),
                            side: Side::Buy,
                        },
                    ],
                })),
            }];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<KrakenTrades>(test.input);
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
