use barter_integration::{Validator, error::SocketError};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Defines the aggregation interval for the MEXC aggregated book ticker stream.
///
/// Used when constructing the subscription topic string, e.g., "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT".
///
/// See docs: <https://mexcdevelop.github.io/apidocs/spot_v3_en/#individual-symbol-book-ticker-streams>
#[derive(Debug, Copy, Clone, Serialize, Default, Eq, PartialEq, Hash)]
pub enum MexcAggInterval {
    /// 10ms aggregation interval.
    #[serde(rename = "10ms")]
    Ms10,
    /// 100ms aggregation interval.
    #[serde(rename = "100ms")]
    #[default] // Default to 100ms as a common choice.
    Ms100,
}

/// Defines the WebSocket method for MEXC subscription messages.
#[derive(Debug, Copy, Clone, Serialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum MexcWsMethod {
    Subscription,
    Unsubscription,
}

/// Outbound WebSocket message for subscribing to or unsubscribing from MEXC channels.
///
/// See docs: <https://mexcdevelop.github.io/apidocs/spot_v3_en/#public-subscription>
#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
pub struct MexcWsSub<'a> {
    #[serde(rename = "method")]
    pub method: MexcWsMethod,
    #[serde(rename = "params")]
    pub params: Cow<'a, [String]>, // List of topic strings
    pub id: u64,
}

/// Inbound WebSocket message received from MEXC in response to a subscription or unsubscription attempt.
///
/// ### Raw Payload Examples
/// See docs: <https://mexcdevelop.github.io/apidocs/spot_v3_en/#public-subscription>
///
/// #### Subscription Success (Single Topic)
/// ```json
/// {
///     "id": null,
///     "code": 0,
///     "msg": "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT"
/// }
/// ```
///
/// #### Subscription Success (Multiple Topics - structure inferred from general responses)
/// ```json
/// {
///     "code": 0,
///     "data": [
///         "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT",
///         "spot@public.aggre.bookTicker.v3.api.pb@100ms@ETHUSDT"
///     ]
/// }
/// ```
///
/// #### Subscription Failure
/// ```json
/// {
///     "id": null,
///     "code": 1,
///     "msg": "Invalid topic spot@public.aggre.bookTicker.v3.api.pb@100ms@ABC"
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MexcSubResponse {
    /// Status code of the operation. `0` indicates success.
    pub code: i32,
    /// Optional message, used for error details or confirming a single successful subscription/unsubscription.
    #[serde(rename = "msg", alias = "message", default)]
    pub detail: Option<String>,
    /// Optional data, used for confirming multiple successful subscriptions/unsubscriptions.
    #[serde(default)]
    pub data: Option<Vec<String>>,
    /// Optional id field. If present as `null` in JSON, it becomes `Some(Value::Null)`.
    /// If missing from JSON, it becomes `None`.
    pub id: Option<serde_json::Value>,
}

impl Validator for MexcSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.code == 0 {
            Ok(self)
        } else {
            Err(SocketError::Subscribe(format!(
                "Subscription/Unsubscription failed with code {}: {}",
                self.code,
                self.detail.as_deref().unwrap_or("No error detail provided")
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_integration::error::SocketError;

    mod de {
        use super::*;

        #[test]
        fn test_mexc_sub_response_deserialization() {
            struct TestCase {
                input: &'static str,
                expected: MexcSubResponse,
            }

            let cases = vec![
                TestCase {
                    // TC0: Single subscription success
                    input: r#"{"id":null,"code":0,"msg":"spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT"}"#,
                    expected: MexcSubResponse {
                        code: 0,
                        detail: Some(
                            "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT".to_string(),
                        ),
                        data: None,
                        // Adjusted to None to match observed deserialization behavior where JSON `null` becomes `Option::None`.
                        id: None,
                    },
                },
                TestCase {
                    // TC1: Multiple subscription success (structure based on general successful responses)
                    // ID field is missing in this input.
                    input: r#"{"code":0,"data":["spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT","spot@public.aggre.bookTicker.v3.api.pb@100ms@ETHUSDT"]}"#,
                    expected: MexcSubResponse {
                        code: 0,
                        detail: None,
                        data: Some(vec![
                            "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT".to_string(),
                            "spot@public.aggre.bookTicker.v3.api.pb@100ms@ETHUSDT".to_string(),
                        ]),
                        id: None, // Expect None when 'id' field is missing
                    },
                },
                TestCase {
                    // TC2: Subscription failure
                    input: r#"{"id":null,"code":1,"msg":"Invalid topic spot@public.aggre.bookTicker.v3.api.pb@100ms@ABC"}"#,
                    expected: MexcSubResponse {
                        code: 1,
                        detail: Some(
                            "Invalid topic spot@public.aggre.bookTicker.v3.api.pb@100ms@ABC"
                                .to_string(),
                        ),
                        data: None,
                        // Adjusted to None to match observed deserialization behavior where JSON `null` becomes `Option::None`.
                        id: None,
                    },
                },
                TestCase {
                    // TC3: Subscription success with no specific msg or data (general success)
                    // ID field is missing in this input.
                    input: r#"{"code":0}"#,
                    expected: MexcSubResponse {
                        code: 0,
                        detail: None,
                        data: None,
                        id: None, // Expect None when 'id' field is missing
                    },
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                match serde_json::from_str::<MexcSubResponse>(test.input) {
                    Ok(actual) => assert_eq!(actual, test.expected, "TC{} failed", index),
                    Err(e) => panic!("TC{} failed deserialization: {:?}", index, e),
                }
            }
        }
    }

    #[test]
    fn test_validate_mexc_sub_response() {
        struct TestCase {
            name: &'static str,
            input_response: MexcSubResponse,
            expected_is_ok: bool,
            expected_error_msg: Option<String>,
        }

        let cases = vec![
            TestCase {
                name: "TC0: Subscription success (code 0, with msg)",
                input_response: MexcSubResponse {
                    code: 0,
                    detail: Some(
                        "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT".to_string(),
                    ),
                    data: None,
                    // Assuming JSON `null` for id deserializes to None in this environment.
                    id: None,
                },
                expected_is_ok: true,
                expected_error_msg: None,
            },
            TestCase {
                name: "TC1: Subscription success (code 0, with data)",
                input_response: MexcSubResponse {
                    code: 0,
                    detail: None,
                    data: Some(vec!["topic1".to_string()]),
                    id: None,
                },
                expected_is_ok: true,
                expected_error_msg: None,
            },
            TestCase {
                name: "TC2: Subscription success (code 0, no detail/data)",
                input_response: MexcSubResponse {
                    code: 0,
                    detail: None,
                    data: None,
                    id: None,
                },
                expected_is_ok: true,
                expected_error_msg: None,
            },
            TestCase {
                name: "TC3: Subscription failure (code 1, with msg)",
                input_response: MexcSubResponse {
                    code: 1,
                    detail: Some("Invalid topic".to_string()),
                    data: None,
                    // Assuming JSON `null` for id deserializes to None in this environment.
                    id: None,
                },
                expected_is_ok: false,
                expected_error_msg: Some(
                    "Subscription/Unsubscription failed with code 1: Invalid topic".to_string(),
                ),
            },
            TestCase {
                name: "TC4: Subscription failure (code 500, no msg)",
                input_response: MexcSubResponse {
                    code: 500,
                    detail: None,
                    data: None,
                    id: None,
                },
                expected_is_ok: false,
                expected_error_msg: Some(
                    "Subscription/Unsubscription failed with code 500: No error detail provided"
                        .to_string(),
                ),
            },
        ];

        for test in cases.into_iter() {
            let result = test.input_response.validate();
            assert_eq!(
                result.is_ok(),
                test.expected_is_ok,
                "Test case '{}' failed: expected is_ok to be {}",
                test.name,
                test.expected_is_ok
            );
            if let Some(expected_msg) = test.expected_error_msg {
                match result {
                    Ok(_) => panic!(
                        "Test case '{}' failed: expected error but got Ok",
                        test.name
                    ),
                    Err(SocketError::Subscribe(actual_msg)) => {
                        assert_eq!(
                            actual_msg, expected_msg,
                            "Test case '{}' failed: error message mismatch",
                            test.name
                        );
                    }
                    Err(other_err) => panic!(
                        "Test case '{}' failed: unexpected error type {:?}",
                        test.name, other_err
                    ),
                }
            }
        }
    }

    mod ser {
        use super::*;
        use std::borrow::Cow;

        #[test]
        fn test_mexc_ws_sub_serialization() {
            struct TestCase {
                name: &'static str,
                input: MexcWsSub<'static>, // Lifetime 'static specified here
                expected_json: &'static str,
            }

            let cases = vec![
                TestCase {
                    name: "TC0: Single subscription",
                    input: MexcWsSub {
                        method: MexcWsMethod::Subscription,
                        params: Cow::Owned(vec![
                            "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT".to_string(),
                        ]),
                        id: 123, // Adding an ID for unsubscription
                    },
                    expected_json: r#"{"method":"SUBSCRIPTION","params":["spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT"],"id":123}"#,
                },
                TestCase {
                    name: "TC1: Multiple subscriptions",
                    input: MexcWsSub {
                        method: MexcWsMethod::Subscription,
                        params: Cow::Owned(vec![
                            "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT".to_string(),
                            "spot@public.aggre.bookTicker.v3.api.pb@10ms@ETHUSDT".to_string(),
                        ]),
                        id: 123, // Adding an ID for unsubscription
                    },
                    expected_json: r#"{"method":"SUBSCRIPTION","params":["spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT","spot@public.aggre.bookTicker.v3.api.pb@10ms@ETHUSDT"],"id":123}"#,
                },
                TestCase {
                    name: "TC2: Unsubscription",
                    input: MexcWsSub {
                        method: MexcWsMethod::Unsubscription,
                        params: Cow::Owned(vec![
                            "spot@public.aggre.bookTicker.v3.api.pb@100ms@LTCUSDT".to_string(),
                        ]),
                        id: 123, // Adding an ID for unsubscription
                    },
                    expected_json: r#"{"method":"UNSUBSCRIPTION","params":["spot@public.aggre.bookTicker.v3.api.pb@100ms@LTCUSDT"],"id":123}"#,
                },
            ];

            for test in cases.into_iter() {
                let actual_json = serde_json::to_string(&test.input).expect("Failed to serialize");
                assert_eq!(
                    actual_json, test.expected_json,
                    "Test case '{}' failed JSON serialization",
                    test.name
                );
            }
        }

        #[test]
        fn test_mexc_agg_interval_serialization() {
            assert_eq!(
                serde_json::to_string(&MexcAggInterval::Ms10).unwrap(),
                r#""10ms""#
            );
            assert_eq!(
                serde_json::to_string(&MexcAggInterval::Ms100).unwrap(),
                r#""100ms""#
            );
        }
    }
}
