//! Market definitions for Bitget exchange.

use crate::Identifier;
use jackbot_integration::subscription::SubscriptionId;

/// Bitget market identifier (e.g., BTCUSDT for spot, BTCUSDT_UMCBL for futures).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BitgetMarket(pub String);

impl From<&str> for BitgetMarket {
    fn from(market_id: &str) -> Self {
        Self(market_id.to_string())
    }
}

impl From<String> for BitgetMarket {
    fn from(market_id: String) -> Self {
        Self(market_id)
    }
}

impl AsRef<str> for BitgetMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Identifier<BitgetMarket> for BitgetMarket {
    fn id(&self) -> BitgetMarket {
        self.clone()
    }
}

impl std::fmt::Display for BitgetMarket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitget_market_from_str() {
        let market = BitgetMarket::from("BTCUSDT");
        assert_eq!(market.0, "BTCUSDT");
    }

    #[test]
    fn test_bitget_market_from_string() {
        let market = BitgetMarket::from(String::from("BTCUSDT_UMCBL"));
        assert_eq!(market.0, "BTCUSDT_UMCBL");
    }

    #[test]
    fn test_bitget_market_display() {
        let market = BitgetMarket::from("ETHUSDT");
        assert_eq!(format!("{}", market), "ETHUSDT");
    }
}
