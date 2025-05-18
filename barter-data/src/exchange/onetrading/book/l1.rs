use crate::{
    books::Level,
    event::{MarketEvent, MarketIter},
    exchange::onetrading::message::OneTradingPayload,
    subscription::book::OrderBookL1,
};
use barter_instrument::exchange::ExchangeId;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Type alias for OneTrading L1 orderbook message (top of book)
pub type OneTradingOrderBookL1Message = OneTradingPayload<OneTradingBookTickerData>;

/// Data for OneTrading book ticker (L1 orderbook) message
///
/// ### Raw Payload Example
/// ```json
/// {
///   "type": "BOOK_TICKER",
///   "channel": {
///     "name": "BOOK_TICKER",
///     "instrument": "BTC_EUR"
///   },
///   "time": 1732051274299000000,
///   "data": {
///     "instrument": "BTC_EUR",
///     "bestBidPrice": "51200.5",
///     "bestBidAmount": "0.12345",
///     "bestAskPrice": "51210.0",
///     "bestAskAmount": "0.09876",
///     "timestamp": 1732051274298000000
///   }
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OneTradingBookTickerData {
    /// Instrument identifier
    pub instrument: String,
    
    /// Best bid price
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub bestBidPrice: f64,
    
    /// Best bid amount
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub bestBidAmount: f64,
    
    /// Best ask price
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub bestAskPrice: f64,
    
    /// Best ask amount
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub bestAskAmount: f64,
    
    /// Timestamp in nanoseconds
    #[serde(
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub timestamp: chrono::DateTime<Utc>,
}

impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, OneTradingOrderBookL1Message)>
    for MarketIter<InstrumentKey, OrderBookL1>
{
    fn from((exchange, instrument, message): (ExchangeId, InstrumentKey, OneTradingOrderBookL1Message)) -> Self {
        Self(
            vec![Ok(MarketEvent {
                time_exchange: message.data.timestamp,
                time_received: Utc::now(),
                exchange,
                instrument: instrument.clone(),
                kind: OrderBookL1 {
                    last_update_time: message.data.timestamp,
                    best_bid: Some(Level {
                        price: Decimal::from_f64_retain(message.data.bestBidPrice).unwrap_or_default(),
                        amount: Decimal::from_f64_retain(message.data.bestBidAmount).unwrap_or_default(),
                    }),
                    best_ask: Some(Level {
                        price: Decimal::from_f64_retain(message.data.bestAskPrice).unwrap_or_default(),
                        amount: Decimal::from_f64_retain(message.data.bestAskAmount).unwrap_or_default(),
                    }),
                },
            })]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_integration::subscription::SubscriptionId;
    use chrono::TimeZone;
    use smol_str::ToSmolStr;

    #[test]
    fn test_onetrading_book_ticker_deserialization() {
        let json = r#"{
            "type": "BOOK_TICKER",
            "channel": {
                "name": "BOOK_TICKER",
                "instrument": "BTC_EUR"
            },
            "time": 1732051274299000000,
            "data": {
                "instrument": "BTC_EUR",
                "bestBidPrice": "51200.5",
                "bestBidAmount": "0.12345",
                "bestAskPrice": "51210.0",
                "bestAskAmount": "0.09876",
                "timestamp": 1732051274298000000
            }
        }"#;

        let book_ticker: Result<OneTradingOrderBookL1Message, _> = serde_json::from_str(json);
        assert!(book_ticker.is_ok(), "Failed to deserialize book ticker: {:?}", book_ticker.err());
        
        let book_ticker = book_ticker.unwrap();
        assert_eq!(book_ticker.kind, "BOOK_TICKER");
        assert_eq!(book_ticker.subscription_id, SubscriptionId("BOOK_TICKER|BTC_EUR".to_smolstr()));
        
        // Verify the timestamp conversion
        let expected_time = Utc.timestamp_nanos(1732051274299000000);
        assert_eq!(book_ticker.time, expected_time);
        
        // Verify the book ticker data
        assert_eq!(book_ticker.data.instrument, "BTC_EUR");
        assert_eq!(book_ticker.data.bestBidPrice, 51200.5);
        assert_eq!(book_ticker.data.bestBidAmount, 0.12345);
        assert_eq!(book_ticker.data.bestAskPrice, 51210.0);
        assert_eq!(book_ticker.data.bestAskAmount, 0.09876);
        
        let expected_timestamp = Utc.timestamp_nanos(1732051274298000000);
        assert_eq!(book_ticker.data.timestamp, expected_timestamp);
    }
    
    #[test]
    fn test_onetrading_book_ticker_conversion() {
        // Create a sample book ticker
        let book_ticker = OneTradingOrderBookL1Message {
            kind: "BOOK_TICKER".to_string(),
            subscription_id: SubscriptionId("BOOK_TICKER|BTC_EUR".to_smolstr()),
            time: Utc.timestamp_nanos(1732051274299000000),
            data: OneTradingBookTickerData {
                instrument: "BTC_EUR".to_string(),
                bestBidPrice: 51200.5,
                bestBidAmount: 0.12345,
                bestAskPrice: 51210.0,
                bestAskAmount: 0.09876,
                timestamp: Utc.timestamp_nanos(1732051274298000000),
            },
        };
        
        // Test conversion to MarketIter
        let market_iter: MarketIter<String, OrderBookL1> = 
            (ExchangeId::OneTrading, "BTC_EUR".to_string(), book_ticker).into();
        
        // Verify the converted data
        let events = market_iter.0;
        assert_eq!(events.len(), 1);
        
        let event = events[0].as_ref().unwrap();
        assert_eq!(event.exchange, ExchangeId::OneTrading);
        assert_eq!(event.instrument, "BTC_EUR");
        assert_eq!(event.time_exchange, Utc.timestamp_nanos(1732051274298000000));
        
        let orderbook = &event.kind;
        assert_eq!(orderbook.bid.price, 51200.5);
        assert_eq!(orderbook.bid.amount, 0.12345);
        assert_eq!(orderbook.ask.price, 51210.0);
        assert_eq!(orderbook.ask.amount, 0.09876);
    }
}