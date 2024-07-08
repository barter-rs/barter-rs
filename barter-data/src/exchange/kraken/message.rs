use crate::Identifier;
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};

/// [`Kraken`](super::Kraken) message variants that can be received over
/// [`WebSocket`](barter_integration::protocol::websocket::WebSocket).
///
/// ### Raw Payload Examples
/// See docs: <https://docs.kraken.com/websockets/#overview>
///
/// #### OrderBookL1
/// See docs: <https://docs.kraken.com/websockets/#message-spread>
/// ```json
/// [
///     0,
///     [
///         "5698.40000",
///         "5700.00000",
///         "1542057299.545897",
///         "1.01234567",
///         "0.98765432"
///     ],
///     "spread",
///     "XBT/USD"
/// ]
/// ```
///
/// #### Trades
/// See docs: <https://docs.kraken.com/websockets/#message-trade>
/// ```json
/// [
///     0,
///     [
///         [
///             "5541.20000",
///             "0.15850568",
///             "1534614057.321597",
///             "s",
///             "l",
///             ""
///         ],
///         [
///         "6060.00000",
///         "0.02455000",
///         "1534614057.324998",
///         "b",
///         "l",
///         ""
///         ]
///     ],
///     "trade",
///     "XBT/USD"
/// ]
/// ```
///
/// #### Heartbeat
/// See docs: <https://docs.kraken.com/websockets/#message-heartbeat>
/// ```json
/// {
///   "event": "heartbeat"
/// }
/// ```
///
/// #### KrakenError Generic
/// See docs: <https://docs.kraken.com/websockets/#errortypes>
/// ```json
/// {
///     "errorMessage": "Malformed request",
///     "event": "error"
/// }
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum KrakenMessage<T> {
    Data(T),
    Event(KrakenEvent),
}

impl<T> Identifier<Option<SubscriptionId>> for KrakenMessage<T>
where
    T: Identifier<Option<SubscriptionId>>,
{
    fn id(&self) -> Option<SubscriptionId> {
        match self {
            Self::Data(data) => data.id(),
            Self::Event(_) => None,
        }
    }
}

/// [`Kraken`](super::Kraken) messages received over the WebSocket which are not subscription data.
///
/// eg/ [`Kraken`](super::Kraken) sends a [`KrakenEvent::Heartbeat`] if no subscription traffic
/// has been sent within the last second.
///
/// See [`KrakenMessage`] for full raw payload examples.
///
/// See docs: <https://docs.kraken.com/websockets/#message-heartbeat>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum KrakenEvent {
    Heartbeat,
    Error(KrakenError),
}

/// [`Kraken`](super::Kraken) generic error message String received over the WebSocket.
///
/// Note that since the [`KrakenError`] is only made up of a renamed message String field, it can
/// be used flexible as a [`KrakenSubResponse::Error`](super::subscription::KrakenSubResponse)
/// or as a generic error received over the WebSocket while subscriptions are active.
///
/// See [`KrakenMessage`] for full raw payload examples.
///
/// See docs: <https://docs.kraken.com/websockets/#errortypes> <br>
/// See docs: <https://docs.kraken.com/websockets/#message-subscriptionStatus>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenError {
    #[serde(alias = "errorMessage")]
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::error::SocketError;

        #[test]
        fn test_kraken_message_event() {
            struct TestCase {
                input: &'static str,
                expected: Result<KrakenMessage<()>, SocketError>,
            }

            let tests = vec![
                TestCase {
                    // TC0: valid KrakenTrades::Event(KrakenEvent::Heartbeat)
                    input: r#"{"event": "heartbeat"}"#,
                    expected: Ok(KrakenMessage::Event(KrakenEvent::Heartbeat)),
                },
                TestCase {
                    // TC1: valid KrakenTrades::Event(KrakenEvent::Error(KrakenError))
                    input: r#"{"errorMessage": "Malformed request", "event": "error"}"#,
                    expected: Ok(KrakenMessage::Event(KrakenEvent::Error(KrakenError {
                        message: "Malformed request".to_string(),
                    }))),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<KrakenMessage<()>>(test.input);
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
}
