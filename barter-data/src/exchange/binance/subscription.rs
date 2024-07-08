use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// [`Binance`](super::Binance) subscription response message.
///
/// ### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#live-subscribing-unsubscribing-to-streams>
/// #### Subscription Success
/// ```json
/// {
///     "id":1,
///     "result":null
/// }
/// ```
///
/// #### Subscription Failure
/// ```json
/// {
///     "id":1,
///     "result":[]
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceSubResponse {
    result: Option<Vec<String>>,
    id: u32,
}

impl Validator for BinanceSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.result.is_none() {
            Ok(self)
        } else {
            Err(SocketError::Subscribe(
                "received failure subscription response".to_owned(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_binance_sub_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<BinanceSubResponse, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is Subscribed
                    input: r#"{"id":1,"result":null}"#,
                    expected: Ok(BinanceSubResponse {
                        result: None,
                        id: 1,
                    }),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"{"result": [], "id": 1}"#,
                    expected: Ok(BinanceSubResponse {
                        result: Some(vec![]),
                        id: 1,
                    }),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<BinanceSubResponse>(test.input);
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
    fn test_validate_binance_sub_response() {
        struct TestCase {
            input_response: BinanceSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: BinanceSubResponse {
                    result: None,
                    id: 1,
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: BinanceSubResponse {
                    result: Some(vec![]),
                    id: 1,
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
