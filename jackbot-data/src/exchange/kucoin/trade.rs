//! Trade event parsing and types for Kucoin exchange.

use crate::subscription::trade::PublicTrade;
use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
};
use chrono::{DateTime, Utc};
use jackbot_instrument::{Side, exchange::ExchangeId};
use jackbot_integration::subscription::SubscriptionId;
use serde::{Deserialize, Serialize};

/// Kucoin trade message as received from the WebSocket API.
/// See: https://docs.kucoin.com/#public-market-data
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct KucoinTrade {
    pub symbol: String,
    pub sequence: String,
    pub price: String,
    pub size: String,
    pub side: String,
    pub time: u64,
}

impl KucoinTrade {
    pub fn to_public_trade(&self) -> Option<PublicTrade> {
        let price = self.price.parse::<f64>().ok()?;
        let amount = self.size.parse::<f64>().ok()?;
        let side = match self.side.to_ascii_lowercase().as_str() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            _ => return None,
        };
        Some(PublicTrade {
            id: self.sequence.clone(),
            price,
            amount,
            side,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackbot_instrument::Side;

    #[test]
    fn test_kucoin_trade_to_public_trade() {
        let json = r#"{
            \"symbol\": \"BTC-USDT\",
            \"sequence\": \"123456789\",
            \"price\": \"42000.5\",
            \"size\": \"0.01\",
            \"side\": \"sell\",
            \"time\": 1717000000000
        }"#;
        let trade: KucoinTrade = serde_json::from_str(json).unwrap();
        let public = trade.to_public_trade().unwrap();
        assert_eq!(public.price, 42000.5);
        assert_eq!(public.amount, 0.01);
        assert_eq!(public.side, Side::Sell);
        assert_eq!(public.id, "123456789");
    }
}
