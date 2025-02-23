use super::BinanceChannel;
use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
    exchange::ExchangeSub,
    subscription::trade::PublicTrade,
};
use barter_instrument::{Side, exchange::ExchangeId};
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Binance real-time trade message.
///
/// Note:
/// For [`BinanceFuturesUsd`](super::futures::BinanceFuturesUsd) this real-time stream is
/// undocumented.
///
/// See discord: <https://discord.com/channels/910237311332151317/923160222711812126/975712874582388757>
///
/// ### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#trade-streams>
/// #### Spot Side::Buy Trade
/// ```json
/// {
///     "e":"trade",
///     "E":1649324825173,
///     "s":"ETHUSDT",
///     "t":1000000000,
///     "p":"10000.19",
///     "q":"0.239000",
///     "b":10108767791,
///     "a":10108764858,
///     "T":1749354825200,
///     "m":false,
///     "M":true
/// }
/// ```
///
/// #### FuturePerpetual Side::Sell Trade
/// ```json
/// {
///     "e": "trade",
///     "E": 1649839266194,
///     "T": 1749354825200,
///     "s": "ETHUSDT",
///     "t": 1000000000,
///     "p":"10000.19",
///     "q":"0.239000",
///     "X": "MARKET",
///     "m": true
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceTrade {
    #[serde(alias = "s", deserialize_with = "de_trade_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(
        alias = "T",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    #[serde(alias = "t")]
    pub id: u64,
    #[serde(alias = "p", deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    #[serde(alias = "q", deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,
    #[serde(alias = "m", deserialize_with = "de_side_from_buyer_is_maker")]
    pub side: Side,
}

impl Identifier<Option<SubscriptionId>> for BinanceTrade {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BinanceTrade)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange_id, instrument, trade): (ExchangeId, InstrumentKey, BinanceTrade)) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: trade.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: PublicTrade {
                id: trade.id.to_string(),
                price: trade.price,
                amount: trade.amount,
                side: trade.side,
            },
        })])
    }
}

/// Deserialize a [`BinanceTrade`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@trade|BTCUSDT").
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BinanceChannel::TRADES, market)).id())
}

/// Deserialize a [`BinanceTrade`] "buyer_is_maker" boolean field to a Barter [`Side`].
///
/// Variants:
/// buyer_is_maker => Side::Sell
/// !buyer_is_maker => Side::Buy
pub fn de_side_from_buyer_is_maker<'de, D>(deserializer: D) -> Result<Side, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(|buyer_is_maker| {
        if buyer_is_maker {
            Side::Sell
        } else {
            Side::Buy
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use std::time::Duration;

        use barter_integration::{de::datetime_utc_from_epoch_duration, error::SocketError};
        use serde::de::Error;

        use super::*;

        #[test]
        fn test_binance_trade() {
            struct TestCase {
                input: &'static str,
                expected: Result<BinanceTrade, SocketError>,
            }

            let tests = vec![
                TestCase {
                    // TC0: Spot trade valid
                    input: r#"
                    {
                        "e":"trade","E":1649324825173,"s":"ETHUSDT","t":1000000000,
                        "p":"10000.19","q":"0.239000","b":10108767791,"a":10108764858,
                        "T":1749354825200,"m":false,"M":true
                    }
                    "#,
                    expected: Ok(BinanceTrade {
                        subscription_id: SubscriptionId::from("@trade|ETHUSDT"),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1749354825200,
                        )),
                        id: 1000000000,
                        price: 10000.19,
                        amount: 0.239000,
                        side: Side::Buy,
                    }),
                },
                TestCase {
                    // TC1: Spot trade malformed w/ "yes" is_buyer_maker field
                    input: r#"{
                        "e":"trade","E":1649324825173,"s":"ETHUSDT","t":1000000000,
                        "p":"10000.19000000","q":"0.239000","b":10108767791,"a":10108764858,
                        "T":1649324825173,"m":"yes","M":true
                    }"#,
                    expected: Err(SocketError::Deserialise {
                        error: serde_json::Error::custom(""),
                        payload: "".to_owned(),
                    }),
                },
                TestCase {
                    // TC2: FuturePerpetual trade w/ type MARKET
                    input: r#"
                    {
                        "e": "trade","E": 1649839266194,"T": 1749354825200,"s": "ETHUSDT",
                        "t": 1000000000,"p":"10000.19","q":"0.239000","X": "MARKET","m": true
                    }
                    "#,
                    expected: Ok(BinanceTrade {
                        subscription_id: SubscriptionId::from("@trade|ETHUSDT"),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1749354825200,
                        )),
                        id: 1000000000,
                        price: 10000.19,
                        amount: 0.239000,
                        side: Side::Sell,
                    }),
                },
                TestCase {
                    // TC3: FuturePerpetual trade w/ type LIQUIDATION
                    input: r#"
                    {
                        "e": "trade","E": 1649839266194,"T": 1749354825200,"s": "ETHUSDT",
                        "t": 1000000000,"p":"10000.19","q":"0.239000","X": "LIQUIDATION","m": false
                    }
                    "#,
                    expected: Ok(BinanceTrade {
                        subscription_id: SubscriptionId::from("@trade|ETHUSDT"),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1749354825200,
                        )),
                        id: 1000000000,
                        price: 10000.19,
                        amount: 0.239000,
                        side: Side::Buy,
                    }),
                },
                TestCase {
                    // TC4: FuturePerpetual trade w/ type LIQUIDATION
                    input: r#"{
                        "e": "trade","E": 1649839266194,"T": 1749354825200,"s": "ETHUSDT",
                        "t": 1000000000,"p":"10000.19","q":"0.239000","X": "INSURANCE_FUND","m": false
                    }"#,
                    expected: Ok(BinanceTrade {
                        subscription_id: SubscriptionId::from("@trade|ETHUSDT"),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1749354825200,
                        )),
                        id: 1000000000,
                        price: 10000.19,
                        amount: 0.239000,
                        side: Side::Buy,
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BinanceTrade>(test.input);
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
