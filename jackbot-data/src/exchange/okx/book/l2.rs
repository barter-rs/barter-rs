use crate::{
    Identifier,
    books::{Canonicalizer, Level, OrderBook},
    event::{MarketEvent, MarketIter},
    exchange::{okx::channel::OkxChannel, subscription::ExchangeSub},
    subscription::book::{OrderBookEvent, OrderBooksL2},
};
use chrono::{DateTime, Utc};
use jackbot_instrument::exchange::ExchangeId;
use jackbot_integration::subscription::SubscriptionId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// OKX real-time OrderBook Level2 message.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OkxOrderBookL2 {
    #[serde(alias = "instId", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(default = "Utc::now")]
    pub time: DateTime<Utc>,
    #[serde(alias = "bids")]
    pub bids: Vec<(Decimal, Decimal)>,
    #[serde(alias = "asks")]
    pub asks: Vec<(Decimal, Decimal)>,
    #[serde(default)]
    pub action: Option<String>, // "snapshot" or "update"
}

impl Identifier<Option<SubscriptionId>> for OkxOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl Canonicalizer for OkxOrderBookL2 {
    fn canonicalize(&self, timestamp: DateTime<Utc>) -> OrderBook {
        OrderBook::new(
            0,                              // OKX doesn't provide sequence numbers
            Some(self.time.max(timestamp)), // Use the latest timestamp
            self.bids
                .iter()
                .map(|(price, amount)| Level::new(*price, *amount)),
            self.asks
                .iter()
                .map(|(price, amount)| Level::new(*price, *amount)),
        )
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, OkxOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from((exchange_id, instrument, book): (ExchangeId, InstrumentKey, OkxOrderBookL2)) -> Self {
        let order_book = book.canonicalize(Utc::now());

        // Determine if this is a snapshot or update based on the action field
        // OKX uses "snapshot" for initial snapshots and "update" for incremental updates
        let event = match book.action.as_deref() {
            Some("snapshot") => OrderBookEvent::Snapshot(order_book),
            _ => OrderBookEvent::Update(order_book),
        };

        Self(vec![Ok(MarketEvent {
            time_exchange: book.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: event,
        })])
    }
}

/// Deserialize an OkxOrderBookL2 "instId" as the associated SubscriptionId.
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((OkxChannel::ORDER_BOOK_L2, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_okx_order_book_l2_update() {
        let input = r#"{"instId":"BTC-USDT","bids":[["30000.0","1.0"]],"asks":[["30010.0","2.0"]],"action":"update"}"#;
        let book: OkxOrderBookL2 = serde_json::from_str(input).unwrap();
        assert_eq!(book.bids[0], (dec!(30000.0), dec!(1.0)));
        assert_eq!(book.asks[0], (dec!(30010.0), dec!(2.0)));
        assert_eq!(book.action, Some("update".to_string()));
    }

    #[test]
    fn test_okx_order_book_l2_snapshot() {
        let input = r#"{"instId":"BTC-USDT","bids":[["30000.0","1.0"],["29990.0","2.0"]],"asks":[["30010.0","2.0"],["30020.0","3.0"]],"action":"snapshot"}"#;
        let book: OkxOrderBookL2 = serde_json::from_str(input).unwrap();
        assert_eq!(book.bids[0], (dec!(30000.0), dec!(1.0)));
        assert_eq!(book.bids[1], (dec!(29990.0), dec!(2.0)));
        assert_eq!(book.asks[0], (dec!(30010.0), dec!(2.0)));
        assert_eq!(book.asks[1], (dec!(30020.0), dec!(3.0)));
        assert_eq!(book.action, Some("snapshot".to_string()));
    }
}
