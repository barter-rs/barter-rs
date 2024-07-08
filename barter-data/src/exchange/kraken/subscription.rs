use super::message::KrakenError;
use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// [`Kraken`](super::Kraken) message received in response to WebSocket subscription requests.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.kraken.com/websockets/#message-subscriptionStatus>
/// #### Subscription Trade Success
/// ```json
/// {
///   "channelID": 10001,
///   "channelName": "ticker",
///   "event": "subscriptionStatus",
///   "pair": "XBT/EUR",
///   "status": "subscribed",
///   "subscription": {
///     "name": "ticker"
///   }
/// }
/// ```
///
/// #### Subscription Trade Failure
/// ```json
/// {
///   "errorMessage": "Subscription name invalid",
///   "event": "subscriptionStatus",
///   "pair": "XBT/USD",
///   "status": "error",
///   "subscription": {
///     "name": "trades"
///   }
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum KrakenSubResponse {
    Subscribed {
        #[serde(alias = "channelID")]
        channel_id: u64,
        #[serde(alias = "channelName")]
        channel_name: String,
        pair: String,
    },
    Error(KrakenError),
}

impl Validator for KrakenSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            KrakenSubResponse::Subscribed { .. } => Ok(self),
            KrakenSubResponse::Error(error) => Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                error.message
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_kraken_sub_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<KrakenSubResponse, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is Subscribed
                    input: r#"
                    {
                        "channelID": 10001,
                        "channelName": "ticker",
                        "event": "subscriptionStatus",
                        "pair": "XBT/EUR",
                        "status": "subscribed",
                        "subscription": {
                            "name": "ticker"
                        }
                    }
                    "#,
                    expected: Ok(KrakenSubResponse::Subscribed {
                        channel_id: 10001,
                        channel_name: "ticker".to_string(),
                        pair: "XBT/EUR".to_string(),
                    }),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"
                    {
                        "errorMessage": "Subscription name invalid",
                        "event": "subscriptionStatus",
                        "pair": "XBT/USD",
                        "status": "error",
                        "subscription": {
                            "name": "trades"
                        }
                    }
                    "#,
                    expected: Ok(KrakenSubResponse::Error(KrakenError {
                        message: "Subscription name invalid".to_string(),
                    })),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<KrakenSubResponse>(test.input);
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
    }

    #[test]
    fn test_kraken_sub_response_validate() {
        struct TestCase {
            input_response: KrakenSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: KrakenSubResponse::Subscribed {
                    channel_id: 10001,
                    channel_name: "ticker".to_string(),
                    pair: "XBT/EUR".to_string(),
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: KrakenSubResponse::Error(KrakenError {
                    message: "Subscription name invalid".to_string(),
                }),
                is_valid: false,
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = test.input_response.validate().is_ok();
            assert_eq!(actual, test.is_valid, "TestCase {} failed", index);
        }
    }
}
