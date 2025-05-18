use crate::{
    Identifier,
    exchange::onetrading::OneTrading,
    subscription::{
        Subscription,
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a [`OneTrading`]
/// channel to be subscribed to.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct OneTradingChannel(pub &'static str);

impl OneTradingChannel {
    /// [`OneTrading`] real-time trades channel name.
    pub const TRADES: Self = Self("PRICE_TICKS");

    /// [`OneTrading`] real-time OrderBook Level1 (top of books) channel name.
    pub const ORDER_BOOK_L1: Self = Self("BOOK_TICKER");

    /// [`OneTrading`] OrderBook Level2 channel name.
    pub const ORDER_BOOK_L2: Self = Self("ORDERBOOK");
}

impl<Instrument> Identifier<OneTradingChannel>
    for Subscription<OneTrading, Instrument, PublicTrades>
{
    fn id(&self) -> OneTradingChannel {
        OneTradingChannel::TRADES
    }
}

impl<Instrument> Identifier<OneTradingChannel>
    for Subscription<OneTrading, Instrument, OrderBooksL1>
{
    fn id(&self) -> OneTradingChannel {
        OneTradingChannel::ORDER_BOOK_L1
    }
}

impl<Instrument> Identifier<OneTradingChannel>
    for Subscription<OneTrading, Instrument, OrderBooksL2>
{
    fn id(&self) -> OneTradingChannel {
        OneTradingChannel::ORDER_BOOK_L2
    }
}

impl AsRef<str> for OneTradingChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}