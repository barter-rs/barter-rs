//! Trade event parsing and types for Crypto.com exchange.
//!
//! Provides conversion to [`PublicTrade`](crate::subscription::trade::PublicTrade).
use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
    subscription::trade::PublicTrade,
};
use chrono::{DateTime, Utc};
use jackbot_instrument::{Side, exchange::ExchangeId};
use jackbot_integration::subscription::SubscriptionId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CryptocomTrade {
    pub s: String,
    pub p: String,
    pub q: String,
    pub side: String,
    pub t: u64,
    #[serde(default)]
    pub trade_id: u64,
}

impl CryptocomTrade {
    pub fn to_public_trade(&self) -> Option<PublicTrade> {
        let price = self.p.parse::<f64>().ok()?;
        let amount = self.q.parse::<f64>().ok()?;
        let side = match self.side.to_ascii_lowercase().as_str() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            _ => return None,
        };
        Some(PublicTrade {
            id: self.trade_id.to_string(),
            price,
            amount,
            side,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CryptocomTrades {
    pub data: Vec<CryptocomTrade>,
    #[serde(skip)]
    pub subscription_id: Option<SubscriptionId>,
}

impl Identifier<Option<SubscriptionId>> for CryptocomTrades {
    fn id(&self) -> Option<SubscriptionId> {
        self.subscription_id.clone()
    }
}

impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, CryptocomTrades)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange, instrument, trades): (ExchangeId, InstrumentKey, CryptocomTrades)) -> Self {
        trades
            .data
            .into_iter()
            .filter_map(|trade| trade.to_public_trade())
            .map(|public| {
                Ok(MarketEvent {
                    time_exchange: Utc::now(),
                    time_received: Utc::now(),
                    exchange,
                    instrument: instrument.clone(),
                    kind: public,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackbot_instrument::Side;

    #[test]
    fn test_cryptocom_trade_to_public_trade() {
        let json = r#"{
            \"s\": \"BTC_USDT\",
            \"p\": \"42000.5\",
            \"q\": \"0.01\",
            \"side\": \"sell\",
            \"t\": 1717000000000,
            \"trade_id\": 42
        }"#;
        let trade: CryptocomTrade = serde_json::from_str(json).unwrap();
        let public = trade.to_public_trade().unwrap();
        assert_eq!(public.price, 42000.5);
        assert_eq!(public.amount, 0.01);
        assert_eq!(public.side, Side::Sell);
        assert_eq!(public.id, "42");
    }
}
