// use super::{channel::IbkrChannel, market::IbkrMarket};
// use crate::exchange::subscription::ExchangeSub;
use super::unsolicited::{
    authentication_status::IbkrStsResponse,
    system_connection::IbkrSystemResponse,
};
use barter_integration::{error::SocketError, Validator};
use serde::Deserialize;
use tracing::info;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
#[serde(tag = "topic", rename_all = "lowercase")]
pub enum IbkrSubResponse {
    #[serde(rename = "system")]
    System(IbkrSystemResponse),
    #[serde(rename = "sts")]
    Sts(IbkrStsResponse),
    Error {
        code: String,
        #[serde(rename = "msg")]
        message: String,
    },
}

impl Validator for IbkrSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            Self::System(_system) => {
                info!("IbkrSubResponse::System");
                Ok(self)
            },
            Self::Sts(_sts) => {
                info!("IbkrSubResponse::Sts");
                Ok(self)
            },
            Self::Error { code, message } => Err(SocketError::Subscribe(format!(
                "received failure subscription response code: {code} with message: {message}",
            ))),
        }
    }
}

// TODO: fix the failed subscription tests
#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use crate::exchange::ibkr::unsolicited::authentication_status::StsArgs;

        use super::*;

        #[test]
        fn test_ibkr_subscription_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<IbkrSubResponse, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is system connection success
                    input: r#"
                        {
                            "topic":"system",
                            "success":"some_username",
                            "isFT":false,
                            "isPaper":false
                        }
                    "#,
                    expected: Ok(IbkrSubResponse::System(IbkrSystemResponse{
                        // topic: String::from("system"),
                        username: String::from("some_username"),
                        is_ft: false,
                        is_paper: false,
                    })),
                },
                TestCase {
                    // TC1: input response is authentication status success
                    input: r#"
                        {
                            "topic":"sts",
                            "args": {
                                "authenticated": true,
                                "competing": false,
                                "message": "",
                                "fail": "",
                                "serverName": "some_servername",
                                "serverVersion": "Build 10.28.0c, Apr 1, 2024 6:35:40 PM",
                                "username": "some_username"
                            }
                        }
                    "#,
                    expected: Ok(IbkrSubResponse::Sts(IbkrStsResponse{
                        args: StsArgs {
                            authenticated: true,
                            competing: false,
                            message: String::new(),
                            fail: String::new(),
                            server_name: String::from("some_servername"),
                            server_version: String::from("Build 10.28.0c, Apr 1, 2024 6:35:40 PM"),
                            username: String::from("some_username")
                        }
                    })),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<IbkrSubResponse>(test.input);
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
    fn test_validate_ibkr_sub_response() {
        struct TestCase {
            input_response: IbkrSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is subscription success
                input_response: IbkrSubResponse::System(IbkrSystemResponse{
                    // topic: String::from("system"),
                    username: String::from("some_username"),
                    is_ft: false,
                    is_paper: false,
                }),
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: IbkrSubResponse::Error {
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
