//! Public trades stream and normalization for Hyperliquid.

use crate::subscription::trade::PublicTrade;
use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
};
use chrono::{DateTime, Utc};
use jackbot_instrument::{Side, exchange::ExchangeId};
use jackbot_integration::subscription::SubscriptionId;
use serde::{Deserialize, Serialize};

/// Hyperliquid trade message as received from the WebSocket API.
/// See: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct HyperliquidTrade {
    pub coin: String,
    pub side: String,
    pub px: String,
    pub sz: String,
    pub hash: String,
    pub time: u64,
    pub tid: u64,
    pub users: [String; 2],
}

impl HyperliquidTrade {
    pub fn to_public_trade(&self) -> Option<PublicTrade> {
        let price = self.px.parse::<f64>().ok()?;
        let amount = self.sz.parse::<f64>().ok()?;
        let side = match self.side.to_ascii_lowercase().as_str() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            _ => return None,
        };
        let time = chrono::NaiveDateTime::from_timestamp_millis(self.time as i64)
            .map(|dt| DateTime::<Utc>::from_utc(dt, Utc))?;
        Some(PublicTrade {
            id: self.tid.to_string(),
            price,
            amount,
            side,
            // Optionally add more fields if PublicTrade supports them
        })
    }
}

/// Wrapper for a batch of Hyperliquid trades, as received from the WebSocket API.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HyperliquidTrades {
    pub data: Vec<HyperliquidTrade>,
    #[serde(skip)]
    pub subscription_id: Option<SubscriptionId>, // Not present in HL API, but required for trait
}

impl Identifier<Option<SubscriptionId>> for HyperliquidTrades {
    fn id(&self) -> Option<SubscriptionId> {
        self.subscription_id.clone()
    }
}

impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, HyperliquidTrades)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from(
        (exchange, instrument, trades): (ExchangeId, InstrumentKey, HyperliquidTrades),
    ) -> Self {
        trades
            .data
            .into_iter()
            .filter_map(|trade| trade.to_public_trade())
            .map(|public_trade| {
                Ok(MarketEvent {
                    time_exchange: chrono::Utc::now(), // HL trade has ms timestamp, can parse if needed
                    time_received: chrono::Utc::now(),
                    exchange,
                    instrument: instrument.clone(),
                    kind: public_trade,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscription::trade::PublicTrade;
    use jackbot_instrument::Side;

    #[test]
    fn test_hyperliquid_trade_to_public_trade() {
        let json = r#"{
            "coin": "BTC",
            "side": "buy",
            "px": "42000.5",
            "sz": "0.01",
            "hash": "abc123",
            "time": 1717000000000,
            "tid": 123456789,
            "users": ["user1", "user2"]
        }"#;
        let trade: HyperliquidTrade = serde_json::from_str(json).unwrap();
        let public = trade.to_public_trade().unwrap();
        assert_eq!(public.price, 42000.5);
        assert_eq!(public.amount, 0.01);
        assert_eq!(public.side, Side::Buy);
        assert_eq!(public.id, "123456789");
    }
}
