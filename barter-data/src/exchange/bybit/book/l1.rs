use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{bybit::channel::BybitChannel, subscription::ExchangeSub, ExchangeId},
    subscription::book::{Level, OrderBookL1},
    Identifier,
};
use barter_integration::model::{Exchange, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BybitOrderBookL1 {
    pub topic: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub ts: DateTime<Utc>,
    #[serde(rename = "type")]
    pub update_type: String,
    pub data: BybitOrderBookL1Data,
    pub cts: u64,
}

#[derive(Debug, Deserialize)]
pub struct BybitOrderBookL1Data {
    pub s: String,
    #[serde(deserialize_with = "de_ob_l1_level")]
    pub b: [f64; 2], // [price, amount]
    #[serde(deserialize_with = "de_ob_l1_level")]
    pub a: [f64; 2], // [price, amount]
    pub u: u64,
    pub seq: u64,
}

pub fn de_ob_l1_level<'de, D>(deserializer: D) -> Result<[f64; 2], D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let mut value = [0.0, 0.0];
    if let Ok(level) = <[[&str; 2]; 1] as Deserialize>::deserialize(deserializer) {
        value[0] = level[0][0].parse().map_err(serde::de::Error::custom)?;
        value[1] = level[0][1].parse().map_err(serde::de::Error::custom)?;
    }

    Ok(value)
}

impl Identifier<Option<SubscriptionId>> for BybitOrderBookL1 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(ExchangeSub::from((BybitChannel::ORDER_BOOK_L1, &self.data.s)).id())
    }
}

impl<InstrumentId> From<(ExchangeId, InstrumentId, BybitOrderBookL1)>
    for MarketIter<InstrumentId, OrderBookL1>
{
    fn from((exchange_id, instrument, book): (ExchangeId, InstrumentId, BybitOrderBookL1)) -> Self {
        Self(vec![Ok(MarketEvent {
            exchange_time: book.ts,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: OrderBookL1 {
                last_update_time: book.ts,
                best_bid: Level::new(book.data.b[0], book.data.b[1]),
                best_ask: Level::new(book.data.a[0], book.data.a[1]),
            },
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bybit_order_book_l1() {
        let input = r#"
        {
            "topic": "orderbook.1.BTCUSDT",
            "ts": 1724458107654,
            "type": "delta",
            "data": {
                "s": "BTCUSDT",
                "b": [["64055.75", "0.503641"]],
                "a": [["64055.76", "0.123456"]],
                "u": 37965267,
                "seq": 38244420107
            },
            "cts": 1724458107650
        }
        "#;
        let actual: BybitOrderBookL1 = serde_json::from_str(input).unwrap();

        assert_eq!(actual.topic, "orderbook.1.BTCUSDT");
        assert_eq!(actual.ts.timestamp_millis(), 1724458107654);
        assert_eq!(actual.update_type, "delta");
        assert_eq!(actual.data.s, "BTCUSDT");
        assert_eq!(actual.data.b[0], 64055.75);
        assert_eq!(actual.data.b[1], 0.503641);
        assert_eq!(actual.data.a[0], 64055.76);
        assert_eq!(actual.data.a[1], 0.123456);
        assert_eq!(actual.data.u, 37965267);
        assert_eq!(actual.data.seq, 38244420107);
        assert_eq!(actual.cts, 1724458107650);

        // Test the Identifier implementation
        assert_eq!(
            actual.id(),
            Some(SubscriptionId::from("orderbook.1|BTCUSDT"))
        );

        // Test the From implementation
        let market_iter: MarketIter<String, OrderBookL1> =
            (ExchangeId::BybitSpot, "BTCUSDT".to_string(), actual).into();

        if let Some(Ok(market_event)) = market_iter.0.get(0) {
            assert_eq!(market_event.instrument, "BTCUSDT");
            let OrderBookL1 {
                best_bid, best_ask, ..
            } = &market_event.kind;
            assert_eq!(best_bid.price, 64055.75);
            assert_eq!(best_bid.amount, 0.503641);
            assert_eq!(best_ask.price, 64055.76);
            assert_eq!(best_ask.amount, 0.123456);
        } else {
            panic!("Failed to get market event from MarketIter");
        }
    }
}
