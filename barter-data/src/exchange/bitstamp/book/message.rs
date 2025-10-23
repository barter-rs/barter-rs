use barter_instrument::exchange::ExchangeId;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    books::{Level, OrderBook},
    event::{MarketEvent, MarketIter},
    exchange::bitstamp::message::BitstampPayload,
    subscription::book::OrderBookEvent,
};

pub type BitstampOrderBookL2Message = BitstampPayload<BitstampOrderBookL2Inner>;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct BitstampOrderBookL2Inner {
    #[serde(
        rename = "microtimestamp",
        deserialize_with = "barter_integration::de::de_str"
    )]
    pub timestamp: u64,

    #[serde(rename = "bids")]
    pub bids: Vec<BitstampLevel>,

    #[serde(rename = "asks")]
    pub asks: Vec<BitstampLevel>,
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BitstampOrderBookL2Inner)>
    for MarketEvent<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, payload): (ExchangeId, InstrumentKey, BitstampOrderBookL2Inner),
    ) -> Self {
        let time_received = Utc::now();
        let time_exchange = Utc.timestamp_micros(payload.timestamp as i64).unwrap();

        Self {
            time_exchange,
            time_received,
            exchange,
            instrument,
            kind: OrderBookEvent::from(payload),
        }
    }
}

impl From<BitstampOrderBookL2Inner> for OrderBookEvent {
    fn from(snapshot: BitstampOrderBookL2Inner) -> Self {
        let time_exchange = Utc.timestamp_micros(snapshot.timestamp as i64).unwrap();

        Self::Snapshot(OrderBook::new(
            snapshot.timestamp,
            Some(time_exchange),
            snapshot.bids,
            snapshot.asks,
        ))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct BitstampLevel {
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

impl From<BitstampLevel> for Level {
    fn from(level: BitstampLevel) -> Self {
        Self {
            price: level.price,
            amount: level.amount,
        }
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BitstampOrderBookL2Message)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, update): (ExchangeId, InstrumentKey, BitstampOrderBookL2Message),
    ) -> Self {
        let time_exchange = Utc.timestamp_micros(update.data.timestamp as i64).unwrap();

        Self(vec![Ok(MarketEvent {
            time_exchange,
            time_received: Utc::now(),
            exchange,
            instrument,
            kind: OrderBookEvent::Update(OrderBook::new(
                update.data.timestamp,
                Some(time_exchange),
                update.data.bids,
                update.data.asks,
            )),
        })])
    }
}

#[cfg(test)]
mod tests {
    mod de {
        use crate::exchange::bitstamp::book::message::BitstampOrderBookL2Message;

        #[test]
        fn test_bitstamp_l2_orderbook() {
            let input = r#"{"data":{"timestamp":"1761237030","microtimestamp":"1761237030014407","bids":[["109880","0.50331929"],["109879","0.15000000"],["109878","0.00000000"],["109876","0.09956720"],["109874","0.00000000"],["109871","0.00000000"],["109869","0.22753041"],["109868","0.00000000"],["109866","0.18201045"],["109865","0.04439898"],["109862","0.00000000"],["109861","0.00000000"],["109860","0.39275611"],["109859","0.50000000"],["109855","0.00000000"],["109853","0.00000000"],["109851","0.00000000"],["109846","0.00000000"],["109842","0.93085695"],["109841","0.00000000"],["109831","0.45180000"],["109829","0.00000000"],["109826","0.14721605"],["109806","0.91069027"],["109793","0.00000000"],["109785","0.00154810"],["109778","0.00000000"],["109765","0.02734089"],["109760","0.01238909"],["109643","0.01080000"]],"asks":[["109881","0.80897787"],["109883","0.00000000"],["109886","0.00000000"],["109887","0.45729028"],["109888","0.00000000"],["109891","0.02046562"],["109898","0.91221600"],["109900","0.50000000"],["109902","0.40043846"],["109906","0.18326988"],["109907","0.00000000"],["109913","0.16608551"],["109918","0.00011764"],["109924","0.06858131"],["109925","0.00000000"],["109958","0.00094118"],["109964","0.00000000"],["109982","0.00000000"]]},"channel":"diff_order_book_btcusd","event":"data"}"#;
            serde_json::from_str::<BitstampOrderBookL2Message>(input).unwrap();
        }
    }
}
