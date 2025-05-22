//! Trade event parsing and types for Gate.io exchange.

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
pub struct GateioTrade {
    pub currency_pair: String,
    pub price: String,
    pub amount: String,
    pub side: String,
    pub time: u64,
    #[serde(default)]
    pub id: String,
}

impl GateioTrade {
    pub fn to_public_trade(&self) -> Option<PublicTrade> {
        let price = self.price.parse::<f64>().ok()?;
        let amount = self.amount.parse::<f64>().ok()?;
        let side = match self.side.to_ascii_lowercase().as_str() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            _ => return None,
        };
        Some(PublicTrade { id: self.id.clone(), price, amount, side })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GateioTrades {
    pub data: Vec<GateioTrade>,
    #[serde(skip)]
    pub subscription_id: Option<SubscriptionId>,
}

impl Identifier<Option<SubscriptionId>> for GateioTrades {
    fn id(&self) -> Option<SubscriptionId> {
        self.subscription_id.clone()
    }
}

impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, GateioTrades)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange, instrument, trades): (ExchangeId, InstrumentKey, GateioTrades)) -> Self {
        trades
            .data
            .into_iter()
            .filter_map(|t| t.to_public_trade())
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
    fn test_gateio_trade_to_public_trade() {
        let json = r#"{
            \"currency_pair\": \"BTC_USDT\",
            \"price\": \"42000.5\",
            \"amount\": \"0.01\",
            \"side\": \"buy\",
            \"time\": 1717000000000,
            \"id\": \"abc\"
        }"#;
        let trade: GateioTrade = serde_json::from_str(json).unwrap();
        let public = trade.to_public_trade().unwrap();
        assert_eq!(public.price, 42000.5);
        assert_eq!(public.amount, 0.01);
        assert_eq!(public.side, Side::Buy);
        assert_eq!(public.id, "abc");
    }
}
