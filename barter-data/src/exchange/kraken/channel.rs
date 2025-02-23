use super::Kraken;
use crate::{
    Identifier,
    subscription::{Subscription, book::OrderBooksL1, trade::PublicTrades},
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
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
}

impl<Instrument> Identifier<KrakenChannel> for Subscription<Kraken, Instrument, PublicTrades> {
    fn id(&self) -> KrakenChannel {
        KrakenChannel::TRADES
    }
}

impl<Instrument> Identifier<KrakenChannel> for Subscription<Kraken, Instrument, OrderBooksL1> {
    fn id(&self) -> KrakenChannel {
        KrakenChannel::ORDER_BOOK_L1
    }
}

impl AsRef<str> for KrakenChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
