use super::message::GateioMessage;
use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// Expected [`Gateio`](super::Gateio) [`Subscription`](crate::subscription::Subscription) response
/// type wrapped in the generic [`GateioMessage<T>`](GateioMessage).
pub type GateioSubResponse = GateioMessage<GateioSubResult>;

/// Expected [`Gateio`](super::Gateio) [`Subscription`](crate::subscription::Subscription)
/// response type.
///
/// See [`GateioMessage`](super::message::GateioMessage) for full raw payload examples.
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#server-response>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioSubResult {
    pub status: String,
}

impl Validator for GateioSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self.error {
            None => Ok(self),
            Some(failure) => Err(SocketError::Subscribe(format!(
                "received failure subscription response code: {} with message: {}",
                failure.code, failure.message,
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::gateio::message::GateioError;

    mod de {
        use super::*;

        #[test]
        fn test_gateio_sub_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<GateioSubResponse, SocketError>,
            }

            let tests = vec![TestCase {
                // TC0: input response is Subscribed
                input: r#"
                    {
                        "time": 1606292218,
                        "time_ms": 1606292218231,
                        "channel": "spot.trades",
                        "event": "subscribe",
                        "result": {
                            "status": "success"
                        }
                    }
                    "#,
                expected: Ok(GateioSubResponse {
                    channel: "spot.trades".to_string(),
                    error: None,
                    data: GateioSubResult {
                        status: "success".to_string(),
                    },
                }),
            }];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<GateioSubResponse>(test.input);
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
    fn test_validate_gateio_sub_response() {
        struct TestCase {
            input_response: GateioSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: GateioSubResponse {
                    channel: "spot.trades".to_string(),
                    error: None,
                    data: GateioSubResult {
                        status: "success".to_string(),
                    },
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: GateioSubResponse {
                    channel: "spot.trades".to_string(),
                    error: Some(GateioError {
                        code: 0,
                        message: "".to_string(),
                    }),
                    data: GateioSubResult {
                        status: "not used".to_string(),
                    },
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
