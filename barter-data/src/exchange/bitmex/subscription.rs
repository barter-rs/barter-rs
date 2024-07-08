use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// ### Raw Payload Examples
/// See docs: <https://www.bitmex.com/app/wsAPI#Response-Format>
/// #### Subscription response payload
/// ```json
/// {
///     "success": true,
///     "subscribe": "trade:XBTUSD",
///     "request": {
///         "op":"subscribe",
///         "args":[
///             "trade:XBTUSD"
///         ]
///     }
/// }
///```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitmexSubResponse {
    success: bool,
    subscribe: String,
}

impl Validator for BitmexSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.success {
            Ok(self)
        } else {
            Err(SocketError::Subscribe(format!(
                "received failure subscription response for {} subscription",
                self.subscribe
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_bitmex_sub_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<BitmexSubResponse, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is Subscribed
                    input: r#"
                        {
                            "success": true,
                            "subscribe": "orderBookL2_25:XBTUSD",
                            "request": {
                                "op":"subscribe",
                                "args":[
                                    "orderBookL2_25:XBTUSD"
                                ]
                            }
                        }
                    "#,
                    expected: Ok(BitmexSubResponse {
                        success: true,
                        subscribe: "orderBookL2_25:XBTUSD".to_string(),
                    }),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"
                    {
                        "success": false,
                        "subscribe": "orderBookL2_25:XBTUSD"
                    }
                    "#,
                    expected: Ok(BitmexSubResponse {
                        success: false,
                        subscribe: "orderBookL2_25:XBTUSD".to_string(),
                    }),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<BitmexSubResponse>(test.input);
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
    fn test_validate_bitmex_sub_response() {
        struct TestCase {
            input_response: BitmexSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: BitmexSubResponse {
                    success: true,
                    subscribe: "orderBookL2_25:XBTUSD".to_string(),
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: BitmexSubResponse {
                    success: false,
                    subscribe: "orderBookL2_25:XBTUSD".to_string(),
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
