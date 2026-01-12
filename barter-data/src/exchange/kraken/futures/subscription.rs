use barter_integration::{Validator, error::SocketError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum KrakenFuturesSubResponse {
    Subscribed {
        event: String,
        feed: String,
        product_ids: Vec<String>,
    },
    Error {
        event: String,
        message: String,
    },
}

impl Validator for KrakenFuturesSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self {
            KrakenFuturesSubResponse::Subscribed { event, .. } if event == "subscribed" => Ok(self),
            KrakenFuturesSubResponse::Error { message, .. } => {
                Err(SocketError::Subscribe(message.clone()))
            }
            _ => Err(SocketError::Subscribe("Unknown subscription response".to_string())),
        }
    }
}
