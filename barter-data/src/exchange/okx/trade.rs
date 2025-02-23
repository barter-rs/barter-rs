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

/// Terse type alias for an [`Okx`](super::Okx) real-time trades WebSocket message.
pub type OkxTrades = OkxMessage<OkxTrade>;

/// [`Okx`](super::Okx) market data WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
/// #### Spot Buy Trade
/// ```json
/// {
///   "arg": {
///     "channel": "trades",
///     "instId": "BTC-USDT"
///   },
///   "data": [
///     {
///       "instId": "BTC-USDT",
///       "tradeId": "130639474",
///       "px": "42219.9",
///       "sz": "0.12060306",
///       "side": "buy",
///       "ts": "1630048897897"
///     }
///   ]
/// }
/// ```
///
/// #### Option Call Sell Trade
/// ```json
/// {
///   "arg": {
///     "channel": "trades",
///     "instId": "BTC-USD-231229-35000-C"
///   },
///   "data": [
///     {
///       "instId": "BTC-USD-231229-35000-C",
///       "tradeId": "4",
///       "px": "0.1525",
///       "sz": "21",
///       "side": "sell",
///       "ts": "1681473269025"
///     }
///   ]
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OkxMessage<T> {
    #[serde(
        rename = "arg",
        deserialize_with = "de_okx_message_arg_as_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    pub data: Vec<T>,
}

impl<T> Identifier<Option<SubscriptionId>> for OkxMessage<T> {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

/// [`Okx`](super::Okx) real-time trade WebSocket message.
///
/// See [`OkxMessage`] for full raw payload examples.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel-trades-channel>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OkxTrade {
    #[serde(rename = "tradeId")]
    pub id: String,
    #[serde(rename = "px", deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    #[serde(rename = "sz", deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,
    pub side: Side,
    #[serde(
        rename = "ts",
        deserialize_with = "barter_integration::de::de_str_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
}

impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, OkxTrades)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange, instrument, trades): (ExchangeId, InstrumentKey, OkxTrades)) -> Self {
        trades
            .data
            .into_iter()
            .map(|trade| {
                Ok(MarketEvent {
                    time_exchange: trade.time,
                    time_received: Utc::now(),
                    exchange,
                    instrument: instrument.clone(),
                    kind: PublicTrade {
                        id: trade.id,
                        price: trade.price,
                        amount: trade.amount,
                        side: trade.side,
                    },
                })
            })
            .collect()
    }
}

/// Deserialize an [`OkxMessage`] "arg" field as a Barter [`SubscriptionId`].
fn de_okx_message_arg_as_subscription_id<'de, D>(
    deserializer: D,
) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Arg<'a> {
        channel: &'a str,
        inst_id: &'a str,
    }

    Deserialize::deserialize(deserializer)
        .map(|arg: Arg<'_>| ExchangeSub::from((arg.channel, arg.inst_id)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::{de::datetime_utc_from_epoch_duration, error::SocketError};
        use std::time::Duration;

        #[test]
        fn test_okx_message_trades() {
            let input = r#"
            {
                "arg": {
                    "channel": "trades",
                    "instId": "BTC-USDT"
                },
                "data": [
                    {
                        "instId": "BTC-USDT",
                        "tradeId": "130639474",
                        "px": "42219.9",
                        "sz": "0.12060306",
                        "side": "buy",
                        "ts": "1630048897897"
                    }
                ]
            }
            "#;

            let actual = serde_json::from_str::<OkxTrades>(input);
            let expected: Result<OkxTrades, SocketError> = Ok(OkxTrades {
                subscription_id: SubscriptionId::from("trades|BTC-USDT"),
                data: vec![OkxTrade {
                    id: "130639474".to_string(),
                    price: 42219.9,
                    amount: 0.12060306,
                    side: Side::Buy,
                    time: datetime_utc_from_epoch_duration(Duration::from_millis(1630048897897)),
                }],
            });

            match (actual, expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC failed")
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!(
                        "TC failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                    );
                }
            }
        }
    }
}
