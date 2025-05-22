// GateIo Order Book L2 stub

use crate::{
    Identifier,
    books::{Canonicalizer, Level, OrderBook},
    event::{MarketEvent, MarketIter},
    redis_store::RedisStore,
    subscription::book::{OrderBookEvent, OrderBooksL2},
};
use chrono::{DateTime, Utc};
use jackbot_instrument::exchange::ExchangeId;
use jackbot_integration::subscription::SubscriptionId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Gate.io real-time OrderBook Level2 message.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct GateIoOrderBookL2 {
    #[serde(alias = "currency_pair")]
    pub subscription_id: SubscriptionId,
    #[serde(default = "Utc::now")]
    pub time: DateTime<Utc>,
    #[serde(alias = "bids")]
    pub bids: Vec<(Decimal, Decimal)>,
    #[serde(alias = "asks")]
    pub asks: Vec<(Decimal, Decimal)>,
}

impl Identifier<Option<SubscriptionId>> for GateIoOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl Canonicalizer for GateIoOrderBookL2 {
    fn canonicalize(&self, timestamp: DateTime<Utc>) -> OrderBook {
        let bids = self.bids.iter().map(|(p, a)| Level::new(*p, *a));
        let asks = self.asks.iter().map(|(p, a)| Level::new(*p, *a));
        OrderBook::new(0, Some(timestamp), bids, asks)
    }
}

impl GateIoOrderBookL2 {
    /// Persist this order book snapshot to the provided [`RedisStore`].
    pub fn store_snapshot<Store: RedisStore>(&self, store: &Store) {
        let snapshot = self.canonicalize(self.time);
        store.store_snapshot(ExchangeId::GateIo, self.subscription_id.as_ref(), &snapshot);
    }

    /// Persist this order book update to the provided [`RedisStore`].
    pub fn store_delta<Store: RedisStore>(&self, store: &Store) {
        let delta = OrderBookEvent::Update(self.canonicalize(self.time));
        store.store_delta(ExchangeId::GateIo, self.subscription_id.as_ref(), &delta);
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, GateIoOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, book): (ExchangeId, InstrumentKey, GateIoOrderBookL2),
    ) -> Self {
        let order_book = book.canonicalize(book.time);

        Self(vec![Ok(MarketEvent {
            time_exchange: book.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: OrderBookEvent::Update(order_book),
        })])
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::redis_store::InMemoryStore;
    use rust_decimal_macros::dec;

    #[test]
    fn test_gateio_order_book_l2() {
        let input = r#"{\"currency_pair\":\"BTC_USDT\",\"bids\":[[\"30000.0\",\"1.0\"]],\"asks\":[[\"30010.0\",\"2.0\"]]}"#;
        let book: GateIoOrderBookL2 = serde_json::from_str(input).unwrap();
        assert_eq!(book.bids[0], (dec!(30000.0), dec!(1.0)));
        assert_eq!(book.asks[0], (dec!(30010.0), dec!(2.0)));
    }

    #[test]
    fn test_store_methods() {
        let store = InMemoryStore::new();
        let book = GateIoOrderBookL2 {
            subscription_id: "BTC_USDT".into(),
            time: Utc::now(),
            bids: vec![(dec!(30000.0), dec!(1.0))],
            asks: vec![(dec!(30010.0), dec!(2.0))],
        };
        book.store_snapshot(&store);
        assert!(store.get_snapshot_json(ExchangeId::GateIo, "BTC_USDT").is_some());

        let delta_book = GateIoOrderBookL2 { time: Utc::now(), ..book };
        delta_book.store_delta(&store);
        assert_eq!(store.delta_len(ExchangeId::GateIo, "BTC_USDT"), 1);
    }
}
