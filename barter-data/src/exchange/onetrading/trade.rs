use crate::{
    event::{MarketEvent, MarketIter},
    exchange::onetrading::message::OneTradingPayload,
    subscription::trade::PublicTrade,
};
use barter_instrument::{Side, exchange::ExchangeId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type alias for OneTrading real-time trades WebSocket message.
pub type OneTradingTrade = OneTradingPayload<OneTradingTradeData>;

/// Trade data from OneTrading WebSocket API
///
/// ### Raw Payload Example
/// ```json
/// {
///   "type": "PRICE_TICK",
///   "channel": {
///     "name": "PRICE_TICKS",
///     "instrument": "BTC_EUR"
///   },
///   "time": 1732051274299000000,
///   "data": {
///     "instrument": "BTC_EUR",
///     "price": "51234.5",
///     "amount": "0.00145",
///     "timestamp": 1732051274298000000,
///     "side": "BUY",
///     "id": "trade_123456789"
///   }
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OneTradingTradeData {
    /// Instrument identifier (e.g., "BTC_EUR")
    pub instrument: String,
    
    /// Trade price
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    
    /// Trade amount
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,
    
    /// Trade timestamp in nanoseconds
    #[serde(
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub timestamp: DateTime<Utc>,
    
    /// Trade direction (BUY or SELL)
    #[serde(deserialize_with = "de_side")]
    pub side: Side,
    
    /// Unique trade identifier
    pub id: String,
}

/// Deserialize a string "BUY" or "SELL" into Side enum
pub fn de_side<'de, D>(deserializer: D) -> Result<Side, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "BUY" => Ok(Side::Buy),
        "SELL" => Ok(Side::Sell),
        _ => Err(serde::de::Error::custom(format!("unknown side: {}", s))),
    }
}

impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, OneTradingTrade)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange, instrument, trade): (ExchangeId, InstrumentKey, OneTradingTrade)) -> Self {
        Self(
            vec![Ok(MarketEvent {
                time_exchange: trade.data.timestamp,
                time_received: Utc::now(),
                exchange,
                instrument: instrument.clone(),
                kind: PublicTrade {
                    id: trade.data.id,
                    price: trade.data.price,
                    amount: trade.data.amount,
                    side: trade.data.side,
                },
            })]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_integration::{error::SocketError, subscription::SubscriptionId};
    use chrono::TimeZone;
    use smol_str::ToSmolStr;

    #[test]
    fn test_onetrading_trade_deserialization() {
        let json = r#"{
            "type": "PRICE_TICK",
            "channel": {
                "name": "PRICE_TICKS",
                "instrument": "BTC_EUR"
            },
            "time": 1732051274299000000,
            "data": {
                "instrument": "BTC_EUR",
                "price": "51234.5",
                "amount": "0.00145",
                "timestamp": 1732051274298000000,
                "side": "BUY",
                "id": "trade_123456789"
            }
        }"#;

        let trade: Result<OneTradingTrade, _> = serde_json::from_str(json);
        assert!(trade.is_ok(), "Failed to deserialize trade: {:?}", trade.err());
        
        let trade = trade.unwrap();
        assert_eq!(trade.kind, "PRICE_TICK");
        assert_eq!(trade.subscription_id, SubscriptionId("PRICE_TICKS|BTC_EUR".to_smolstr()));
        
        // Verify the timestamp conversion (from nanoseconds to UTC datetime)
        let expected_time = Utc.timestamp_nanos(1732051274299000000);
        assert_eq!(trade.time, expected_time);
        
        // Verify the trade data
        assert_eq!(trade.data.instrument, "BTC_EUR");
        assert_eq!(trade.data.price, 51234.5);
        assert_eq!(trade.data.amount, 0.00145);
        assert_eq!(trade.data.side, Side::Buy);
        assert_eq!(trade.data.id, "trade_123456789");
        
        let expected_timestamp = Utc.timestamp_nanos(1732051274298000000);
        assert_eq!(trade.data.timestamp, expected_timestamp);
    }
    
    #[test]
    fn test_onetrading_trade_conversion() {
        // Create a sample trade
        let trade = OneTradingTrade {
            kind: "PRICE_TICK".to_string(),
            subscription_id: SubscriptionId("PRICE_TICKS|BTC_EUR".to_smolstr()),
            time: Utc.timestamp_nanos(1732051274299000000),
            data: OneTradingTradeData {
                instrument: "BTC_EUR".to_string(),
                price: 51234.5,
                amount: 0.00145,
                timestamp: Utc.timestamp_nanos(1732051274298000000),
                side: Side::Buy,
                id: "trade_123456789".to_string(),
            },
        };
        
        // Test conversion to MarketIter
        let market_iter: MarketIter<String, PublicTrade> = 
            (ExchangeId::OneTrading, "BTC_EUR".to_string(), trade).into();
        
        // Verify the converted data
        let events = market_iter.0;
        assert_eq!(events.len(), 1);
        
        let event = events[0].as_ref().unwrap();
        assert_eq!(event.exchange, ExchangeId::OneTrading);
        assert_eq!(event.instrument, "BTC_EUR");
        assert_eq!(event.time_exchange, Utc.timestamp_nanos(1732051274298000000));
        
        let trade = &event.kind;
        assert_eq!(trade.id, "trade_123456789");
        assert_eq!(trade.price, 51234.5);
        assert_eq!(trade.amount, 0.00145);
        assert_eq!(trade.side, Side::Buy);
    }
}