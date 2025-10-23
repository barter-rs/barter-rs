use barter_integration::{Validator, error::SocketError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BitstampSubResponse {
    event: String,
    channel: String,
}

impl Validator for BitstampSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.event == "bts:subscription_succeeded" {
            Ok(self)
        } else {
            Err(SocketError::Subscribe(format!(
                "received failure subscription response: {}",
                self.event,
            )))
        }
    }
}
