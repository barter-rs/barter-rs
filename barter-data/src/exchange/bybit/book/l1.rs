use barter_instrument::exchange::ExchangeId;
use chrono::Utc;

use crate::{
    event::{MarketEvent, MarketIter},
    exchange::bybit::message::BybitMessage,
    subscription::book::OrderBookL1,
};

use super::BybitOrderBookMessage;

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BybitMessage)>
    for MarketIter<InstrumentKey, OrderBookL1>
where
    InstrumentKey: Clone,
{
    fn from((exchange_id, instrument, message): (ExchangeId, InstrumentKey, BybitMessage)) -> Self {
        match message {
            BybitMessage::Orderbook(book) => Self::from((exchange_id, instrument, book)),
            _ => Self(vec![]),
        }
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BybitOrderBookMessage)>
    for MarketIter<InstrumentKey, OrderBookL1>
where
    InstrumentKey: Clone,
{
    fn from(
        (exchange, instrument, book): (ExchangeId, InstrumentKey, BybitOrderBookMessage),
    ) -> Self {
        let best_ask = book.data.asks.first().copied().map(Into::into);
        let best_bid = book.data.bids.first().copied().map(Into::into);

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
