//! Channel type and normalization for Hyperliquid.

use serde::{Deserialize, Serialize};

/// Channel identifier for Hyperliquid WebSocket subscriptions.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HyperliquidChannel(pub &'static str);

impl AsRef<str> for HyperliquidChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl HyperliquidChannel {
    pub const ORDER_BOOK_L1: Self = Self("l1");
    pub const ORDER_BOOK_L2: Self = Self("l2");
}

// TODO: Implement channel type and normalization.
