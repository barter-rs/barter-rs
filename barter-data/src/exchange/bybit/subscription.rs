use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// [`Bybit`](super::Bybit) subscription response message.
///
///  ### Raw Payload Examples
///  See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect#understanding-the-subscription-response>
///  #### Subscription Success
/// ```json
/// {
///     "success": true,
///     "ret_msg": "subscribe",
///     "conn_id": "2324d924-aa4d-45b0-a858-7b8be29ab52b",
///     "req_id": "10001",
///     "op": "subscribe"
/// }
/// #### Subscription Failure
/// ```json
/// {
///     "success": false,
///     "ret_msg": "",
///     "conn_id": "2324d924-aa4d-45b0-a858-7b8be29ab52b",
///     "req_id": "10001",
///     "op": "subscribe"
/// }
///
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitResponse {
    pub success: bool,
    #[serde(default)]
    pub ret_msg: BybitReturnMessage,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum BybitReturnMessage {
    #[serde(alias = "")]
    None,
    #[serde(alias = "pong")]
    Pong,
    #[serde(alias = "subscribe")]
    Subscribe,
}

impl Default for BybitReturnMessage {
    fn default() -> Self {
        Self::None
    }
}

impl Validator for BybitResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match self.ret_msg {
            BybitReturnMessage::None | BybitReturnMessage::Subscribe => {
                if self.success {
                    Ok(self)
                } else {
                    Err(SocketError::Subscribe(
                        "received failure subscription response".to_owned(),
                    ))
                }
            }
            _ => Err(SocketError::Subscribe(
                "received other message out of sequence".to_owned(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_bybit_sub_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitResponse, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is Subscribed
                    input: r#"
                        {
                            "success": true,
                            "ret_msg": "subscribe",
                            "conn_id": "2324d924-aa4d-45b0-a858-7b8be29ab52b",
                            "req_id": "10001",
                            "op": "subscribe"
                        }
                    "#,
                    expected: Ok(BybitResponse {
                        success: true,
                        ret_msg: BybitReturnMessage::Subscribe,
                    }),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"
                        {
                            "success": false,
                            "conn_id": "",
                            "op": ""
                        }
                    "#,
                    expected: Ok(BybitResponse {
                        success: false,
                        ret_msg: BybitReturnMessage::None,
                    }),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
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
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }
    }

    #[test]
    fn test_validate_bybit_sub_response() {
        struct TestCase {
            input_response: BybitResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: BybitResponse {
                    success: true,
                    ret_msg: BybitReturnMessage::Subscribe,
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is successful subscription
                input_response: BybitResponse {
                    success: true,
                    ret_msg: BybitReturnMessage::None,
                },
                is_valid: true,
            },
            TestCase {
                // TC2: input response is failed subscription
                input_response: BybitResponse {
                    success: false,
                    ret_msg: BybitReturnMessage::Pong,
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
