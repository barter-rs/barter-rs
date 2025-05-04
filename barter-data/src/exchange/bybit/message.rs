use std::fmt::Debug;

use crate::{Identifier, exchange::bybit::channel::BybitChannel};
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use serde::{
    Deserialize, Serialize,
    de::{Error, Unexpected},
};

/// ### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
/// #### Spot Side::Buy Trade
///```json
/// {
///     "topic": "publicTrade.BTCUSDT",
///     "type": "snapshot",
///     "ts": 1672304486868,
///     "data": [
///         {
///             "T": 1672304486865,
///             "s": "BTCUSDT",
///             "S": "Buy",
///             "v": "0.001",
///             "p": "16578.50",
///             "L": "PlusTick",
///             "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
///             "BT": false
///         }
///     ]
/// }
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct BybitPayload<T> {
    #[serde(alias = "topic", deserialize_with = "de_message_subscription_id")]
    pub subscription_id: SubscriptionId,

    #[serde(rename = "type")]
    pub kind: BybitPayloadKind,

    #[serde(
        alias = "ts",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,

    pub data: T,
}

/// Bybit payload kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BybitPayloadKind {
    Snapshot,
    Delta,
}

/// Deserialize a [`BybitPayload`] "s" (eg/ "publicTrade.BTCUSDT") as the associated
/// [`SubscriptionId`].
///
/// eg/ "publicTrade|BTCUSDT"
pub fn de_message_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let input = <&str as serde::Deserialize>::deserialize(deserializer)?;
    let mut tokens = input.split('.');

    match (tokens.next(), tokens.next(), tokens.next()) {
        (Some("publicTrade"), Some(market), None) => Ok(SubscriptionId::from(format!(
            "{}|{market}",
            BybitChannel::TRADES.0
        ))),
        (Some("orderbook"), Some("1"), Some(market)) => Ok(SubscriptionId::from(format!(
            "{}|{market}",
            BybitChannel::ORDER_BOOK_L1.0,
        ))),
        (Some("orderbook"), Some("50"), Some(market)) => Ok(SubscriptionId::from(format!(
            "{}|{market}",
            BybitChannel::ORDER_BOOK_L2.0,
        ))),
        _ => Err(Error::invalid_value(
            Unexpected::Str(input),
            &"invalid message type expected pattern: <type>.<symbol>",
        )),
    }
}

impl<T> Identifier<Option<SubscriptionId>> for BybitPayload<T> {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

#[cfg(test)]
mod tests {

    mod de {
        use crate::exchange::bybit::subscription::{BybitResponse, BybitReturnMessage};
        use barter_integration::error::SocketError;

        #[test]
        fn test_bybit_pong() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitResponse, SocketError>,
            }

            let tests = vec![
                // TC0: input BybitResponse(Pong) is deserialised
                TestCase {
                    input: r#"
                        {
                            "success": true,
                            "ret_msg": "pong",
                            "conn_id": "0970e817-426e-429a-a679-ff7f55e0b16a",
                            "op": "ping"
                        }
                    "#,
                    expected: Ok(BybitResponse {
                        success: true,
                        ret_msg: BybitReturnMessage::Pong,
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitResponse>(test.input);
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
