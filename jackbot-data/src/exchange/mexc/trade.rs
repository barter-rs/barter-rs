//! Trade event parsing and types for MEXC exchange.
//!
//! Provides simple conversion to the generic [`PublicTrade`](crate::subscription::trade::PublicTrade).
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
pub struct MexcTrade {
    pub symbol: String,
    #[serde(rename = "price")]
    pub price: String,
    #[serde(rename = "quantity")]
    pub quantity: String,
    pub side: String,
    #[serde(rename = "timestamp")]
    pub time: u64,
    #[serde(default)]
    pub id: String,
}

impl MexcTrade {
    pub fn to_public_trade(&self) -> Option<PublicTrade> {
        let price = self.price.parse::<f64>().ok()?;
        let amount = self.quantity.parse::<f64>().ok()?;
        let side = match self.side.to_ascii_lowercase().as_str() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            _ => return None,
        };
        Some(PublicTrade {
            id: self.id.clone(),
            price,
            amount,
            side,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MexcTrades {
    pub data: Vec<MexcTrade>,
    #[serde(skip)]
    pub subscription_id: Option<SubscriptionId>,
}

impl Identifier<Option<SubscriptionId>> for MexcTrades {
    fn id(&self) -> Option<SubscriptionId> {
        self.subscription_id.clone()
    }
}

impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, MexcTrades)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange, instrument, trades): (ExchangeId, InstrumentKey, MexcTrades)) -> Self {
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
    fn test_mexc_trade_to_public_trade() {
        let json = r#"{
            \"symbol\": \"BTC_USDT\",
            \"price\": \"42000.5\",
            \"quantity\": \"0.01\",
            \"side\": \"buy\",
            \"timestamp\": 1717000000000,
            \"id\": \"12345\"
        }"#;
        let trade: MexcTrade = serde_json::from_str(json).unwrap();
        let public = trade.to_public_trade().unwrap();
        assert_eq!(public.price, 42000.5);
        assert_eq!(public.amount, 0.01);
        assert_eq!(public.side, Side::Buy);
        assert_eq!(public.id, "12345");
    }
}
