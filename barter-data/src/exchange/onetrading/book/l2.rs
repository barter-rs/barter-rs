use crate::{
    books::{Level, OrderBook},
    event::{MarketEvent, MarketIter},
    exchange::onetrading::message::OneTradingPayload,
    subscription::book::OrderBookEvent,
};
use barter_instrument::exchange::ExchangeId;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Type alias for OneTrading L2 orderbook message
pub type OneTradingOrderBookL2Message = OneTradingPayload<OneTradingOrderBookData>;

/// Data for OneTrading orderbook (L2 depth) message
///
/// ### Raw Payload Example
/// ```json
/// {
///   "type": "ORDERBOOK",
///   "channel": {
///     "name": "ORDERBOOK",
///     "instrument": "BTC_EUR"
///   },
///   "time": 1732051274299000000,
///   "data": {
///     "instrument": "BTC_EUR",
///     "bids": [
///       ["51200.5", "0.12345"],
///       ["51195.0", "0.22345"],
///       ["51190.0", "0.32345"]
///     ],
///     "asks": [
///       ["51210.0", "0.09876"],
///       ["51215.0", "0.19876"],
///       ["51220.0", "0.29876"]
///     ],
///     "timestamp": 1732051274298000000
///   }
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OneTradingOrderBookData {
    /// Instrument identifier
    pub instrument: String,

    /// Array of bid levels [price, amount]
    #[serde(deserialize_with = "de_orderbook_levels")]
    pub bids: Vec<Level>,

    /// Array of ask levels [price, amount]
    #[serde(deserialize_with = "de_orderbook_levels")]
    pub asks: Vec<Level>,

    /// Timestamp in nanoseconds
    #[serde(deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc")]
    pub timestamp: chrono::DateTime<Utc>,
}

/// Deserialize a vector of [price, amount] string pairs into a vector of Level
fn de_orderbook_levels<'de, D>(deserializer: D) -> Result<Vec<Level>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let string_pairs: Vec<Vec<&str>> = Vec::deserialize(deserializer)?;

    string_pairs
        .into_iter()
        .map(|pair| {
            if pair.len() != 2 {
                return Err(serde::de::Error::custom(
                    "expected exactly 2 elements in price-amount pair",
                ));
            }

            let price = pair[0]
                .parse::<f64>()
                .map_err(|_| serde::de::Error::custom("failed to parse price as float"))?;

            let amount = pair[1]
                .parse::<f64>()
                .map_err(|_| serde::de::Error::custom("failed to parse amount as float"))?;

            Ok(Level {
                price: Decimal::from_f64_retain(price).unwrap_or_default(),
                amount: Decimal::from_f64_retain(amount).unwrap_or_default(),
            })
        })
        .collect()
}

impl<InstrumentKey: Clone> From<(ExchangeId, InstrumentKey, OneTradingOrderBookL2Message)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, message): (ExchangeId, InstrumentKey, OneTradingOrderBookL2Message),
    ) -> Self {
        // Create an OrderBook using the bids and asks
        let orderbook = OrderBook::new(
            0,                            // sequence number (using 0 for the initial snapshot)
            Some(message.data.timestamp), // time_engine
            message.data.bids,
            message.data.asks,
        );

        Self(vec![Ok(MarketEvent {
            time_exchange: message.data.timestamp,
            time_received: Utc::now(),
            exchange,
            instrument: instrument.clone(),
            kind: OrderBookEvent::Snapshot(orderbook),
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_integration::subscription::SubscriptionId;
    use chrono::TimeZone;
    use smol_str::ToSmolStr;

    #[test]
    fn test_onetrading_orderbook_deserialization() {
        let json = r#"{
            "type": "ORDERBOOK",
            "channel": {
                "name": "ORDERBOOK",
                "instrument": "BTC_EUR"
            },
            "time": 1732051274299000000,
            "data": {
                "instrument": "BTC_EUR",
                "bids": [
                    ["51200.5", "0.12345"],
                    ["51195.0", "0.22345"],
                    ["51190.0", "0.32345"]
                ],
                "asks": [
                    ["51210.0", "0.09876"],
                    ["51215.0", "0.19876"],
                    ["51220.0", "0.29876"]
                ],
                "timestamp": 1732051274298000000
            }
        }"#;

        let orderbook: Result<OneTradingOrderBookL2Message, _> = serde_json::from_str(json);
        assert!(
            orderbook.is_ok(),
            "Failed to deserialize orderbook: {:?}",
            orderbook.err()
        );

        let orderbook = orderbook.unwrap();
        assert_eq!(orderbook.kind, "ORDERBOOK");
        assert_eq!(
            orderbook.subscription_id,
            SubscriptionId("ORDERBOOK|BTC_EUR".to_smolstr())
        );

        // Verify the timestamp conversion
        let expected_time = Utc.timestamp_nanos(1732051274299000000);
        assert_eq!(orderbook.time, expected_time);

        // Verify the orderbook data
        assert_eq!(orderbook.data.instrument, "BTC_EUR");

        // Check bids
        assert_eq!(orderbook.data.bids.len(), 3);
        assert_eq!(orderbook.data.bids[0].price, 51200.5);
        assert_eq!(orderbook.data.bids[0].amount, 0.12345);
        assert_eq!(orderbook.data.bids[1].price, 51195.0);
        assert_eq!(orderbook.data.bids[1].amount, 0.22345);
        assert_eq!(orderbook.data.bids[2].price, 51190.0);
        assert_eq!(orderbook.data.bids[2].amount, 0.32345);

        // Check asks
        assert_eq!(orderbook.data.asks.len(), 3);
        assert_eq!(orderbook.data.asks[0].price, 51210.0);
        assert_eq!(orderbook.data.asks[0].amount, 0.09876);
        assert_eq!(orderbook.data.asks[1].price, 51215.0);
        assert_eq!(orderbook.data.asks[1].amount, 0.19876);
        assert_eq!(orderbook.data.asks[2].price, 51220.0);
        assert_eq!(orderbook.data.asks[2].amount, 0.29876);

        let expected_timestamp = Utc.timestamp_nanos(1732051274298000000);
        assert_eq!(orderbook.data.timestamp, expected_timestamp);
    }

    #[test]
    fn test_onetrading_orderbook_conversion() {
        // Create a sample orderbook
        let orderbook = OneTradingOrderBookL2Message {
            kind: "ORDERBOOK".to_string(),
            subscription_id: SubscriptionId("ORDERBOOK|BTC_EUR".to_smolstr()),
            time: Utc.timestamp_nanos(1732051274299000000),
            data: OneTradingOrderBookData {
                instrument: "BTC_EUR".to_string(),
                bids: vec![
                    Level {
                        price: 51200.5,
                        amount: 0.12345,
                    },
                    Level {
                        price: 51195.0,
                        amount: 0.22345,
                    },
                    Level {
                        price: 51190.0,
                        amount: 0.32345,
                    },
                ],
                asks: vec![
                    Level {
                        price: 51210.0,
                        amount: 0.09876,
                    },
                    Level {
                        price: 51215.0,
                        amount: 0.19876,
                    },
                    Level {
                        price: 51220.0,
                        amount: 0.29876,
                    },
                ],
                timestamp: Utc.timestamp_nanos(1732051274298000000),
            },
        };

        // Test conversion to MarketIter
        let market_iter: MarketIter<String, OrderBookL2> =
            (ExchangeId::OneTrading, "BTC_EUR".to_string(), orderbook).into();

        // Verify the converted data
        let events = market_iter.0;
        assert_eq!(events.len(), 1);

        let event = events[0].as_ref().unwrap();
        assert_eq!(event.exchange, ExchangeId::OneTrading);
        assert_eq!(event.instrument, "BTC_EUR");
        assert_eq!(
            event.time_exchange,
            Utc.timestamp_nanos(1732051274298000000)
        );

        let orderbook = &event.kind;

        // Check bids
        assert_eq!(orderbook.bids.len(), 3);
        assert_eq!(orderbook.bids[0].price, 51200.5);
        assert_eq!(orderbook.bids[0].amount, 0.12345);

        // Check asks
        assert_eq!(orderbook.asks.len(), 3);
        assert_eq!(orderbook.asks[0].price, 51210.0);
        assert_eq!(orderbook.asks[0].amount, 0.09876);
    }
}
