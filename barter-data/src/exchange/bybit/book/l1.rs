use barter_instrument::exchange::ExchangeId;
use chrono::Utc;

use crate::{
    books::Level,
    event::{MarketEvent, MarketIter},
    subscription::book::OrderBookL1,
};

use super::BybitOrderBookMessage;

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BybitOrderBookMessage)>
    for MarketIter<InstrumentKey, OrderBookL1>
where
    InstrumentKey: Clone,
{
    fn from(
        (exchange, instrument, book): (ExchangeId, InstrumentKey, BybitOrderBookMessage),
    ) -> Self {
        let best_ask = book.data.asks.first().copied().map(Level::from);
        let best_bid = book.data.bids.first().copied().map(Level::from);

        Self(vec![Ok(MarketEvent {
            time_exchange: book.time,
            time_received: Utc::now(),
            exchange,
            instrument,
            kind: OrderBookL1 {
                last_update_time: book.time,
                best_bid,
                best_ask,
            },
        })])
    }
}
