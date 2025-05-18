use barter_integration::{Validator, error::SocketError};
use serde::{Deserialize, Serialize};

/// [`OneTrading`] subscription response message.
///
/// ### Raw Payload Example
/// ```json
/// {
///     "type": "SUBSCRIPTIONS",
///     "channels": [
///         {
///             "name": "PRICE_TICKS"
///         }
///     ],
///     "time": 1732051274299000000
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OneTradingResponse {
    #[serde(rename = "type")]
    pub kind: OneTradingResponseType,
    pub channels: Vec<OneTradingChannel>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OneTradingChannel {
    pub name: String,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OneTradingResponseType {
    Subscriptions,
    #[serde(alias = "ERROR")]
    Error,
    Pong,
}

impl Validator for OneTradingResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match self.kind {
            OneTradingResponseType::Subscriptions => {
                if !self.channels.is_empty() {
                    Ok(self)
                } else {
                    Err(SocketError::Subscribe(
                        "received empty channels in subscription response".to_owned(),
                    ))
                }
            }
            OneTradingResponseType::Error => Err(SocketError::Subscribe(
                "received error subscription response".to_owned(),
            )),
            OneTradingResponseType::Pong => Err(SocketError::Subscribe(
                "received pong message out of sequence".to_owned(),
            )),
        }
    }
}