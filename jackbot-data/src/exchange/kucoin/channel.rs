//! Channel definitions for Kucoin exchange.

use serde::{Deserialize, Serialize};

/// Channel identifier for Kucoin WebSocket subscriptions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KucoinChannel(pub &'static str);

impl AsRef<str> for KucoinChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl KucoinChannel {
    /// Kucoin real-time Level 2 order book channel name.
    pub const ORDER_BOOK_L2: Self = Self("level2");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_consts() {
        assert_eq!(KucoinChannel::ORDER_BOOK_L2.as_ref(), "level2");
    }
}
