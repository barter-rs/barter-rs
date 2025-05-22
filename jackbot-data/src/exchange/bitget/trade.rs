//! Trade event parsing and types for Bitget exchange.

use crate::subscription::trade::PublicTrade;
use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
    exchange::{ExchangeSub, bitget::channel::BitgetChannel},
};
use chrono::{DateTime, Utc};
use jackbot_instrument::{Side, exchange::ExchangeId};
use jackbot_integration::subscription::SubscriptionId;
use serde::{Deserialize, Serialize};

/// Bitget trade message as received from the WebSocket API.
/// See: https://bitgetlimited.github.io/apidoc/en/mix/#public-trade-channel
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BitgetTrade {
    #[serde(alias = "instId", deserialize_with = "de_trade_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(rename = "tradeId")]
    pub id: String,
    #[serde(alias = "px", deserialize_with = "jackbot_integration::de::de_str")]
    pub price: f64,
    #[serde(alias = "sz", deserialize_with = "jackbot_integration::de::de_str")]
    pub amount: f64,
    pub side: Side,
    #[serde(
        alias = "ts",
        deserialize_with = "jackbot_integration::de::de_str_u64_epoch_ms_as_datetime_utc",
    )]
    pub time: DateTime<Utc>,
}

impl Identifier<Option<SubscriptionId>> for BitgetTrade {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BitgetTrade)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange_id, instrument, trade): (ExchangeId, InstrumentKey, BitgetTrade)) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: trade.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: PublicTrade {
                id: trade.id,
                price: trade.price,
                amount: trade.amount,
                side: trade.side,
            },
        })])
    }
}

/// Deserialize a [`BitgetTrade`] "instId" as the associated [`SubscriptionId`].
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BitgetChannel::TRADES, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackbot_instrument::Side;

    #[test]
    fn test_bitget_trade_deserialize() {
        let json = r#"{
            \"instId\": \"BTCUSDT\",
            \"tradeId\": \"123456789\",
            \"px\": \"42000.5\",
            \"sz\": \"0.01\",
            \"side\": \"buy\",
            \"ts\": \"1717000000000\"
        }"#;
        let trade: BitgetTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.price, 42000.5);
        assert_eq!(trade.amount, 0.01);
        assert_eq!(trade.side, Side::Buy);
        assert_eq!(trade.id, "123456789");
        assert_eq!(trade.subscription_id.as_str(), "trade|BTCUSDT");
    }

    #[test]
    fn test_market_iter_from_trade() {
        let json = r#"{
            \"instId\": \"BTCUSDT\",
            \"tradeId\": \"123456789\",
            \"px\": \"42000.5\",
            \"sz\": \"0.01\",
            \"side\": \"sell\",
            \"ts\": \"1717000000000\"
        }"#;
        let trade: BitgetTrade = serde_json::from_str(json).unwrap();
        let events: MarketIter<String, PublicTrade> =
            (ExchangeId::BitgetSpot, "BTCUSDT".to_string(), trade).into();
        assert_eq!(events.len(), 1);
        let MarketEvent { kind, .. } = events.0.into_iter().next().unwrap().unwrap();
        assert_eq!(kind.price, 42000.5);
        assert_eq!(kind.amount, 0.01);
        assert_eq!(kind.side, Side::Sell);
        assert_eq!(kind.id, "123456789");
    }
}
