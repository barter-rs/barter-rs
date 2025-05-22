use super::Kraken;
use crate::subscription::book::OrderBooksL2;
use crate::{
    Identifier,
    subscription::{Subscription, trade::PublicTrades},
};
use serde::Serialize;

/// Type that defines how to translate a Jackbot [`Subscription`] into a
/// [`Kraken`] channel to be subscribed to.
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct KrakenChannel(pub &'static str);

impl KrakenChannel {
    /// [`Kraken`] real-time trades channel name.
    ///
    /// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
    pub const TRADES: Self = Self("trade");

    /// [`Kraken`] real-time OrderBook Level1 (top of books) channel name.
    ///
    /// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
    pub const ORDER_BOOK_L1: Self = Self("spread");

    /// [`Kraken`] real-time OrderBook Level2 channel name.
    ///
    /// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
    pub const ORDER_BOOK_L2: Self = Self("book");
}

impl<Instrument> Identifier<KrakenChannel> for Subscription<Kraken, Instrument, PublicTrades> {
    fn id(&self) -> KrakenChannel {
        KrakenChannel::TRADES
    }
}

impl<Instrument> Identifier<KrakenChannel> for Subscription<Kraken, Instrument, OrderBooksL2> {
    fn id(&self) -> KrakenChannel {
        KrakenChannel::ORDER_BOOK_L2
    }
}

impl AsRef<str> for KrakenChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
