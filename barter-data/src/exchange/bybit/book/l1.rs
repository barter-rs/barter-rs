use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{bybit::futures::l2::BybitPerpetualsOrderBookL2, ExchangeId},
    subscription::book::{Level, OrderBookL1},
};
use barter_integration::model::Exchange;
use chrono::Utc;

use super::BybitLevel;

impl<InstrumentId> From<(ExchangeId, InstrumentId, BybitPerpetualsOrderBookL2)>
    for MarketIter<InstrumentId, OrderBookL1>
{
    fn from(
        (exchange_id, instrument, book): (ExchangeId, InstrumentId, BybitPerpetualsOrderBookL2),
    ) -> Self {
        let best_bid = book.data.bids.first().unwrap_or(&BybitLevel {
            price: 0.0,
            amount: 0.0,
        });
        let best_ask = book.data.asks.first().unwrap_or(&BybitLevel {
            price: 0.0,
            amount: 0.0,
        });

        Self(vec![Ok(MarketEvent {
            exchange_time: book.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: OrderBookL1 {
                last_update_time: book.time,
                best_bid: Level::from((best_bid.price, best_bid.amount)),
                best_ask: Level::from((best_ask.price, best_ask.amount)),
            },
        })])
    }
}
