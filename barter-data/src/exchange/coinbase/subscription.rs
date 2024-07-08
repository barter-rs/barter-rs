use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// [`Coinbase`](super::Coinbase) WebSocket subscription response.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
/// #### Subscripion Success
/// ```json
/// {
///     "type":"subscriptions",
///     "channels":[
///         {"name":"matches","product_ids":["BTC-USD", "ETH-USD"]}
///     ]
/// }
/// ```
///
/// #### Subscription Failure
/// ```json
/// {
///     "type":"error",
///     "message":"Failed to subscribe",
///     "reason":"GIBBERISH-USD is not a valid product"
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CoinbaseSubResponse {
    #[serde(alias = "subscriptions")]
    Subscribed {
        channels: Vec<CoinbaseChannels>,
    },
    Error {
        reason: String,
    },
}

/// Communicates the [`Coinbase`](super::Coinbase) product_ids (eg/ "ETH-USD") associated with
/// a successful channel (eg/ "matches") subscription.
///
/// See [`CoinbaseSubResponse`] for full raw paylaod examples.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct CoinbaseChannels {
    #[serde(alias = "name")]
    pub channel: String,
    pub product_ids: Vec<String>,
}

impl Validator for CoinbaseSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            CoinbaseSubResponse::Subscribed { .. } => Ok(self),
            CoinbaseSubResponse::Error { reason } => Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                reason
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
        fn test_coinbase_sub_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<CoinbaseSubResponse, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is Subscribed
                    input: r#"
                    {
                        "type":"subscriptions",
                        "channels":[
                            {"name":"matches","product_ids":["BTC-USD", "ETH-USD"]}
                        ]
                    }
                    "#,
                    expected: Ok(CoinbaseSubResponse::Subscribed {
                        channels: vec![CoinbaseChannels {
                            channel: "matches".to_string(),
                            product_ids: vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
                        }],
                    }),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"
                    {
                        "type":"error",
                        "message":"Failed to subscribe",
                        "reason":"GIBBERISH-USD is not a valid product"
                    }
                    "#,
                    expected: Ok(CoinbaseSubResponse::Error {
                        reason: "GIBBERISH-USD is not a valid product".to_string(),
                    }),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<CoinbaseSubResponse>(test.input);
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
    fn test_validate_coinbase_sub_response() {
        struct TestCase {
            input_response: CoinbaseSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: CoinbaseSubResponse::Subscribed {
                    channels: vec![CoinbaseChannels {
                        channel: "matches".to_string(),
                        product_ids: vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
                    }],
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: CoinbaseSubResponse::Error {
                    reason: "GIBBERISH-USD is not a valid product".to_string(),
                },
                is_valid: false,
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = test.input_response.validate().is_ok();
            assert_eq!(actual, test.is_valid, "TestCase {} failed", index);
        }
    }
}
