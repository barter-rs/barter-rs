use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// [`Bitfinex`](super::Bitfinex) platform event detailing the variants expected to be received
/// while connecting and subscribing.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
/// #### Platform Status Online
/// ``` json
/// {
///   "event": "info",
///   "version": VERSION,
///   "platform": {
///     "status": 1
///   }
/// }
/// ```
///
/// #### Subscription Trades Success
/// ``` json
/// {
///   event: "subscribed",
///   channel: "trades",
///   chanId: CHANNEL_ID,
///   symbol: "tBTCUSD"
///   pair: "BTCUSD"
/// }
/// ```
///
/// #### Subscription Failure
/// ``` json
/// {
///    "event": "error",
///    "msg": ERROR_MSG,
///    "code": ERROR_CODE
/// }
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum BitfinexPlatformEvent {
    #[serde(rename = "info")]
    PlatformStatus(BitfinexPlatformStatus),
    Subscribed(BitfinexSubResponse),
    Error(BitfinexError),
}

impl Validator for BitfinexPlatformEvent {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            BitfinexPlatformEvent::PlatformStatus(status) => match status.status {
                Status::Operative => Ok(self),
                Status::Maintenance => Err(SocketError::Subscribe(format!(
                    "exchange version: {} with server_id: {} is in maintenance mode",
                    status.api_version, status.server_id,
                ))),
            },
            BitfinexPlatformEvent::Subscribed(_) => Ok(self),
            BitfinexPlatformEvent::Error(error) => Err(SocketError::Subscribe(format!(
                "received failure subscription response code: {} with message: {}",
                error.code, error.msg,
            ))),
        }
    }
}

/// [`Bitfinex`](super::Bitfinex) platform status message containing the server we are connecting
/// to, the version of the API, and if it is in maintenance mode.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.bitfinex.com/docs/ws-general#info-messages>
/// #### Platform Status Operative
/// ``` json
/// {
///   "event": "info",
///   "version": 2,
///   "serverId": ""
///   "platform": {
///     "status": 1
///   }
/// }
/// ```
///
/// #### Platform Status In Maintenance
/// ``` json
/// {
///   "event": "info",
///   "version": 2,
///   "serverId": ""
///   "platform": {
///     "status": 0
///   }
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexPlatformStatus {
    #[serde(rename = "version")]
    api_version: u8,
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "platform")]
    status: Status,
}

/// [`Bitfinex`](super::Bitfinex) platform [`Status`] indicating if the API is in maintenance mode.
///
/// See [`BitfinexPlatformStatus`] for full raw payload examples.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general#info-messages>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub enum Status {
    Maintenance,
    Operative,
}

/// [`Bitfinex`](super::Bitfinex) subscription success response variants for each channel.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
/// #### Subscription Trades Success
/// ``` json
/// {
///   event: "subscribed",
///   channel: "trades",
///   chanId: CHANNEL_ID,
///   symbol: "tBTCUSD"
///   pair: "BTCUSD"
/// }
/// ```
///
/// #### Subscription Failure
/// ``` json
/// {
///    "event": "error",
///    "msg": ERROR_MSG,
///    "code": ERROR_CODE
/// }
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexSubResponse {
    pub channel: String,
    #[serde(rename = "symbol")]
    pub market: String,
    #[serde(rename = "chanId")]
    pub channel_id: BitfinexChannelId,
}

/// [`Bitfinex`](super::Bitfinex) channel identifier that is used to identify the subscription
/// associated with incoming events. See the module level "SubscriptionId" documentation notes
/// for more details.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general#subscribe-to-channels>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexChannelId(pub u32);

/// [`Bitfinex`](super::Bitfinex) error message that is received if a [`BitfinexSubResponse`]
/// indicates a WebSocket subscription failure.
///
/// ### Subscription Error Codes:
/// 10300: Generic failure
/// 10301: Already subscribed
/// 10302: Unknown channel
///
/// See [`BitfinexPlatformStatus`] for full raw payload examples.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexError {
    msg: String,
    code: u32,
}

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Outer {
            #[serde(deserialize_with = "de_status_from_u8")]
            status: Status,
        }

        // Deserialise Outer struct
        let Outer { status } = Outer::deserialize(deserializer)?;

        Ok(status)
    }
}

/// Deserialize a `u8` as a `Bitfinex` platform [`Status`].
///
/// 0u8 => [`Status::Maintenance`](Status), <br>
/// 1u8 => [`Status::Operative`](Status), <br>
/// other => [`de::Error`]
fn de_status_from_u8<'de, D>(deserializer: D) -> Result<Status, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    match Deserialize::deserialize(deserializer)? {
        0 => Ok(Status::Maintenance),
        1 => Ok(Status::Operative),
        other => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(other as u64),
            &"0 or 1",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_de_bitfinex_platform_event() {
        struct TestCase {
            input: &'static str,
            expected: Result<BitfinexPlatformEvent, SocketError>,
        }

        let cases = vec![
            // TC0: platform status is online
            TestCase {
                input: r#"{"event": "info", "version": 2, "serverId": "5b73a436-19ca-4a15-8160-9069bdd7f181", "platform": { "status": 1 }}"#,
                expected: Ok(BitfinexPlatformEvent::PlatformStatus(
                    BitfinexPlatformStatus {
                        api_version: 2,
                        server_id: "5b73a436-19ca-4a15-8160-9069bdd7f181".to_string(),
                        status: Status::Operative,
                    },
                )),
            },
            // TC1: platform status is offline
            TestCase {
                input: r#"{"event": "info", "version": 2, "serverId": "5b73a436-19ca-4a15-8160-9069bdd7f181", "platform": { "status": 0 }}"#,
                expected: Ok(BitfinexPlatformEvent::PlatformStatus(
                    BitfinexPlatformStatus {
                        api_version: 2,
                        server_id: "5b73a436-19ca-4a15-8160-9069bdd7f181".to_string(),
                        status: Status::Maintenance,
                    },
                )),
            },
            // TC1: successful trades channel subscription
            TestCase {
                input: r#"{"event": "subscribed", "channel": "trades", "chanId": 2203, "symbol": "tBTCUSD", "pair": "BTCUSD"}"#,
                expected: Ok(BitfinexPlatformEvent::Subscribed(BitfinexSubResponse {
                    channel: "trades".to_string(),
                    channel_id: BitfinexChannelId(2203),
                    market: "tBTCUSD".to_owned(),
                })),
            },
            // TC2: Input response is error
            TestCase {
                input: r#"{"event": "error", "msg": "Already subscribed", "code": 10202}"#,
                expected: Ok(BitfinexPlatformEvent::Error(BitfinexError {
                    msg: "Already subscribed".to_owned(),
                    code: 10202,
                })),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<BitfinexPlatformEvent>(test.input);
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

    #[test]
    fn test_bitfinex_platform_sub_response_validate() {
        struct TestCase {
            input: BitfinexPlatformEvent,
            expected: Result<BitfinexPlatformEvent, SocketError>,
        }

        let tests = vec![
            TestCase {
                // TC0: bitfinex server is offline
                input: BitfinexPlatformEvent::PlatformStatus(BitfinexPlatformStatus {
                    api_version: 2,
                    server_id: "server_id".to_string(),
                    status: Status::Maintenance,
                }),
                expected: Err(SocketError::Subscribe(format!(
                    "exchange version: {} with server_id: {} is in maintenance mode",
                    2, "server_id",
                ))),
            },
            TestCase {
                // TC1: bitfinex server is online
                input: BitfinexPlatformEvent::PlatformStatus(BitfinexPlatformStatus {
                    api_version: 2,
                    server_id: "server_id".to_string(),
                    status: Status::Operative,
                }),
                expected: Ok(BitfinexPlatformEvent::PlatformStatus(
                    BitfinexPlatformStatus {
                        api_version: 2,
                        server_id: "server_id".to_string(),
                        status: Status::Operative,
                    },
                )),
            },
            TestCase {
                // TC2: subscription success
                input: BitfinexPlatformEvent::Subscribed(BitfinexSubResponse {
                    channel: "channel".to_string(),
                    market: "market".to_string(),
                    channel_id: BitfinexChannelId(1),
                }),
                expected: Ok(BitfinexPlatformEvent::Subscribed(BitfinexSubResponse {
                    channel: "channel".to_string(),
                    market: "market".to_string(),
                    channel_id: BitfinexChannelId(1),
                })),
            },
            TestCase {
                // TC3: subscription error
                input: BitfinexPlatformEvent::Error(BitfinexError {
                    msg: "error message".to_string(),
                    code: 0,
                }),
                expected: Err(SocketError::Subscribe(format!(
                    "received failure subscription response code: {} with message: {}",
                    0, "error message",
                ))),
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = test.input.validate();
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
