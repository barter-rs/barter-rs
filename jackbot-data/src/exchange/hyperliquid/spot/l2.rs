//! L2 order book stream and normalization for Hyperliquid.

use crate::{
    Identifier,
    books::{Level, OrderBook},
    event::{MarketEvent, MarketIter},
    exchange::{hyperliquid::channel::HyperliquidChannel, subscription::ExchangeSub},
    subscription::book::{OrderBookEvent, OrderBooksL2},
};
use chrono::{DateTime, Utc};
use jackbot_instrument::exchange::ExchangeId;
use jackbot_integration::subscription::SubscriptionId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Hyperliquid real-time OrderBook Level2 message.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct HyperliquidOrderBookL2 {
    #[serde(alias = "coin", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(default = "Utc::now")]
    pub time: DateTime<Utc>,
    #[serde(alias = "bids")]
    pub bids: Vec<(Decimal, Decimal)>,
    #[serde(alias = "asks")]
    pub asks: Vec<(Decimal, Decimal)>,
}

impl Identifier<Option<SubscriptionId>> for HyperliquidOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, HyperliquidOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, book): (ExchangeId, InstrumentKey, HyperliquidOrderBookL2),
    ) -> Self {
        let bids: Vec<Level> = book.bids.iter().map(|(p, a)| Level::new(*p, *a)).collect();
        let asks: Vec<Level> = book.asks.iter().map(|(p, a)| Level::new(*p, *a)).collect();
        Self(vec![Ok(MarketEvent {
            time_exchange: book.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: OrderBookEvent::Update(OrderBook::new(0, None, bids, asks)),
        })])
    }
}

/// Deserialize a HyperliquidOrderBookL2 "coin" as the associated SubscriptionId.
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((HyperliquidChannel::ORDER_BOOK_L2, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_hyperliquid_order_book_l2() {
        let input = r#"{"coin":"BTC","bids":[["30000.0","1.0"]],"asks":[["30010.0","2.0"]]}"#;
        let book: HyperliquidOrderBookL2 = serde_json::from_str(input).unwrap();
        assert_eq!(book.bids[0], (dec!(30000.0), dec!(1.0)));
        assert_eq!(book.asks[0], (dec!(30010.0), dec!(2.0)));
    }
}
