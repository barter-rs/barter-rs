//! Market type and normalization for Hyperliquid.

use serde::{Deserialize, Serialize};

/// Market identifier for Hyperliquid WebSocket subscriptions (e.g., "BTC").
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HyperliquidMarket(pub String);

impl AsRef<str> for HyperliquidMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for HyperliquidMarket {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

// TODO: Implement market type and normalization.
