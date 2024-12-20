use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use barter_integration::Validator;

/// Response type for a [`CoinbaseInternational`] subscription.
/// Example:
/// {
///     "channel": "subscriptions",
///     "client_id": "",
///     "timestamp": "2024-12-20T02:52:29.787710914Z",
///     "sequence_num": 1,
///     "events": [
///         {
///             "subscriptions": {
///                 "ticker_batch": ["EURC-USDC"]
///             }
///         }
///     ]
/// }
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
pub struct CoinbaseInternationalSubResponse {
    pub channel: String,
    pub timestamp: DateTime<Utc>,
    pub events: Vec<SubscriptionEvent>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
pub struct SubscriptionEvent {
    pub subscriptions: BTreeMap<String, Vec<String>>,
}

impl Validator for CoinbaseInternationalSubResponse {
    fn validate(self) -> Result<Self, barter_integration::error::SocketError> {
        if self.channel == "subscriptions" {
            Ok(self)
        } else {
            Err(barter_integration::error::SocketError::Subscribe(
                format!("Unexpected response type: {}", self.channel),
            ))
        }
    }
}