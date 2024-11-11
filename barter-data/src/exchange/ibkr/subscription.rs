// use super::{channel::IbkrChannel, market::IbkrMarket};
use super::unsolicited::{
    account_updates::IbkrAccountResponse, authentication_status::IbkrAuthnStatusResponse,
    system_connection::IbkrSystemResponse, system_heartbeat::IbkrSystemHeartbeat,
};
use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// [`Ibkr`](super::Ibkr) platform event detailing the variants expected to be received
/// while connecting and subscribing.
///
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
// #[serde(tag = "topic")]
#[serde(rename_all = "lowercase")]
#[serde(remote = "IbkrPlatformEvent")]
pub enum IbkrPlatformEvent {
    System(IbkrSystemResponse),
    SystemHeartbeat(IbkrSystemHeartbeat),
    Account(IbkrAccountResponse),
    AuthnStatus(IbkrAuthnStatusResponse),
    Subscribed(IbkrSubResponse),
    Error(IbkrError),
}

impl<'de> Deserialize<'de> for IbkrPlatformEvent {
    fn deserialize<D>(msg: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Value::deserialize(msg) {
            Ok(json) => match json.get("topic") {
                Some(topic) => {
                    match topic.as_str() {
                        Some(s) => match s {
                            "system" => {
                                IbkrSystemResponse::deserialize(json).map(IbkrPlatformEvent::System)
                                    .map_err(serde::de::Error::custom)
                            }
                            "act" => {
                                IbkrAccountResponse::deserialize(json).map(IbkrPlatformEvent::Account)
                                    .map_err(serde::de::Error::custom)
                            }
                            "sts" => {
                                IbkrAuthnStatusResponse::deserialize(json)
                                    .map(IbkrPlatformEvent::AuthnStatus)
                                    .map_err(serde::de::Error::custom)
                            }
                            _ => {
                                if s.starts_with("smd+") {
                                    IbkrSubResponse::deserialize(json)
                                        .map(IbkrPlatformEvent::Subscribed)
                                        .map_err(serde::de::Error::custom)
                                } else {
                                    Err(serde::de::Error::custom(format!(
                                        "unknown topic: {s}"
                                    )))
                                }
                            }
                        },
                        None => Err(serde::de::Error::custom(format!(
                            "error converting topic to string"
                        ))),
                    }
                },
                None => Err(serde::de::Error::custom(format!(
                    "error deserializing topic"
                ))),
            },
            Err(error) => Err(serde::de::Error::custom(format!(
                "error deserializing JSON: {error}"
            ))),
        }
    }
}

impl Validator for IbkrPlatformEvent {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            Self::System(_system) => Ok(self),
            Self::SystemHeartbeat(_system_heartbeat) => Ok(self),
            Self::Account(_act) => Ok(self),
            Self::AuthnStatus(_sts) => Ok(self),
            Self::Subscribed(_sub_response) => Ok(self),
            Self::Error(error) => Err(SocketError::Subscribe(format!(
                "received failure subscription response code: {} with message: {}",
                error.code, error.msg,
            ))),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct IbkrSubResponse {
    #[serde(rename = "topic", deserialize_with = "de_sub_topic")]
    pub channel: String,
    #[serde(rename = "conidEx")]
    pub market: String,
    #[serde(rename = "conid")]
    pub channel_id: IbkrChannelId,
}

/// [`Ibkr`](super::Ibkr) channel identifier that is used to identify the subscription
/// associated with incoming events. See the module level "SubscriptionId" documentation notes
/// for more details.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct IbkrChannelId(pub u32);

/// [`Ibkr`](super::Ibkr) error message that is received if a [`IbkrSubResponse`]
/// indicates a WebSocket subscription failure.
///
/// TODO: add docs, specific codes...
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct IbkrError {
    msg: String,
    code: u32,
}

/// extract sub type from topic (ex: "smd+conidEx..." => "md")
pub fn de_sub_topic<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <String as Deserialize>::deserialize(deserializer)
        .map(|topic| String::from(&topic[1..3]))
}

// TODO: fix the failed subscription tests
#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use crate::exchange::ibkr::unsolicited::authentication_status::AuthnStatusArgs;

        use super::*;

        #[test]
        fn test_ibkr_subscription_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<IbkrPlatformEvent, SocketError>,
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
                    expected: Ok(IbkrPlatformEvent::System(IbkrSystemResponse {
                        username: String::from("some_username"),
                        is_ft: false,
                        is_paper: false,
                    })),
                },
                TestCase {
                    // TC1: input response is system heartbeat
                    input: r#"
                        {
                            "topic":"system",
                            "hb":1729601500848
                        }
                    "#,
                    expected: Ok(IbkrPlatformEvent::SystemHeartbeat(IbkrSystemHeartbeat {
                        hb: 1729601500848,
                    })),
                },
                TestCase {
                    // TC2: input response is authentication status success
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
                    expected: Ok(IbkrPlatformEvent::AuthnStatus(IbkrAuthnStatusResponse {
                        args: AuthnStatusArgs {
                            authenticated: true,
                            competing: false,
                            message: String::new(),
                            fail: String::new(),
                            server_name: String::from("some_servername"),
                            server_version: String::from("Build 10.28.0c, Apr 1, 2024 6:35:40 PM"),
                            username: String::from("some_username"),
                        },
                    })),
                },
                // TOOD: and smd+000000 case to test the serde in this file
                // i.e. the smd+000000 => md on topic field
                // (or maybe do this down below in IbkrSubResponse)
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<IbkrPlatformEvent>(test.input);
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

    // #[test]
    // fn test_validate_ibkr_sub_response() {
    //     struct TestCase {
    //         input_response: IbkrSubResponse,
    //         is_valid: bool,
    //     }

    //     let cases = vec![
    //         TestCase {
    //             // TC0: input response is subscription success
    //             input_response: IbkrSubResponse::System(IbkrSystemResponse{
    //                 // topic: String::from("system"),
    //                 username: String::from("some_username"),
    //                 is_ft: false,
    //                 is_paper: false,
    //             }),
    //             is_valid: true,
    //         },
    //         TestCase {
    //             // TC1: input response is failed subscription
    //             input_response: IbkrSubResponse::Error {
    //                 code: "60012".to_string(),
    //                 message: "Invalid request: {\"op\": \"subscribe\", \"args\":[{ \"channel\" : \"trades\", \"instId\" : \"BTC-USD-191227\"}]}".to_string()
    //             },
    //             is_valid: false,
    //         },
    //     ];

    //     for (index, test) in cases.into_iter().enumerate() {
    //         let actual = test.input_response.validate().is_ok();
    //         assert_eq!(actual, test.is_valid, "TestCase {} failed", index);
    //     }
    // }
}
