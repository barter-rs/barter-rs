//! Subscription logic for Hyperliquid.

use jackbot_integration::{Validator, error::SocketError};
use serde::{Deserialize, Serialize};

/// Subscription response type for Hyperliquid WebSocket API.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HyperliquidSubResponse {
    // TODO: Add fields as needed for subscription validation
}

impl Validator for HyperliquidSubResponse {
    fn validate(self) -> Result<Self, SocketError> {
        Ok(self)
    }
}

// TODO: Implement subscription logic.
