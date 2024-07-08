use super::{channel::OkxChannel, market::OkxMarket};
use crate::exchange::subscription::ExchangeSub;
use barter_integration::{error::SocketError, Validator};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

// Implement custom Serialize to assist aesthetics of <Okx as Connector>::requests() function.
impl Serialize for ExchangeSub<OkxChannel, OkxMarket> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("OkxSubArg", 2)?;
        state.serialize_field("channel", self.channel.as_ref())?;
        state.serialize_field("instId", self.market.as_ref())?;
        state.end()
    }
}

/// [`Okx`](super::Okx) WebSocket subscription response.
///
/// ### Raw Payload Examples
/// #### Subscription Trades Ok Response
/// ```json
/// {
///   "event": "subscribe",
///   "args": {
///     "channel": "trades",
///     "instId": "BTC-USD-191227"
///   }
/// }
/// ```
///
/// #### Subscription Trades Error Response
/// ```json
/// {
///   "event": "error",
///   "code": "60012",
///   "msg": "Invalid request: {\"op\": \"subscribe\", \"args\":[{ \"channel\" : \"trades\", \"instId\" : \"BTC-USD-191227\"}]}"
/// }
/// ```
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum OkxSubResponse {
    #[serde(rename = "subscribe")]
    Subscribed,
    Error {
        code: String,
        #[serde(rename = "msg")]
        message: String,
    },
}

impl Validator for OkxSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match self {
            Self::Subscribed => Ok(self),
            Self::Error { code, message } => Err(SocketError::Subscribe(format!(
                "received failure subscription response code: {code} with message: {message}",
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
        fn test_okx_subscription_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<OkxSubResponse, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is subscription success
                    input: r#"
                {
                    "event": "subscribe",
                    "args": {"channel": "trades", "instId": "BTC-USD-191227"}
                }
                "#,
                    expected: Ok(OkxSubResponse::Subscribed),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"
                {
                    "event": "error",
                    "code": "60012",
                    "msg": "Invalid request: {\"op\": \"subscribe\", \"args\":[{ \"channel\" : \"trades\", \"instId\" : \"BTC-USD-191227\"}]}"
                }
                "#,
                    expected: Ok(OkxSubResponse::Error {
                        code: "60012".to_string(),
                        message: "Invalid request: {\"op\": \"subscribe\", \"args\":[{ \"channel\" : \"trades\", \"instId\" : \"BTC-USD-191227\"}]}".to_string()
                    }),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<OkxSubResponse>(test.input);
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
    fn test_validate_okx_sub_response() {
        struct TestCase {
            input_response: OkxSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is subscription success
                input_response: OkxSubResponse::Subscribed,
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: OkxSubResponse::Error {
                    code: "60012".to_string(),
                    message: "Invalid request: {\"op\": \"subscribe\", \"args\":[{ \"channel\" : \"trades\", \"instId\" : \"BTC-USD-191227\"}]}".to_string()
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
