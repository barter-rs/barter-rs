//! Trade event parsing and types for Bitget exchange.

use crate::subscription::trade::PublicTrade;
use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
};
use chrono::{DateTime, Utc};
use jackbot_instrument::{Side, exchange::ExchangeId};
use jackbot_integration::subscription::SubscriptionId;
use serde::{Deserialize, Serialize};

/// Bitget trade message as received from the WebSocket API.
/// See: https://bitgetlimited.github.io/apidoc/en/mix/#public-trade-channel
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct BitgetTrade {
    pub instId: String,
    pub tradeId: String,
    pub px: String,
    pub sz: String,
    pub side: String,
    pub ts: String,
}

impl BitgetTrade {
    pub fn to_public_trade(&self) -> Option<PublicTrade> {
        let price = self.px.parse::<f64>().ok()?;
        let amount = self.sz.parse::<f64>().ok()?;
        let side = match self.side.to_ascii_lowercase().as_str() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            _ => return None,
        };
        Some(PublicTrade {
            id: self.tradeId.clone(),
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
    fn test_bitget_trade_to_public_trade() {
        let json = r#"{
            \"instId\": \"BTCUSDT\",
            \"tradeId\": \"123456789\",
            \"px\": \"42000.5\",
            \"sz\": \"0.01\",
            \"side\": \"buy\",
            \"ts\": \"1717000000000\"
        }"#;
        let trade: BitgetTrade = serde_json::from_str(json).unwrap();
        let public = trade.to_public_trade().unwrap();
        assert_eq!(public.price, 42000.5);
        assert_eq!(public.amount, 0.01);
        assert_eq!(public.side, Side::Buy);
        assert_eq!(public.id, "123456789");
    }
}
