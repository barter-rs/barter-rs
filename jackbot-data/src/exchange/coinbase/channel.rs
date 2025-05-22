use super::Coinbase;
use crate::subscription::book::OrderBooksL2;
use crate::{
    Identifier,
    subscription::{Subscription, trade::PublicTrades},
};
use serde::Serialize;

/// Type that defines how to translate a Jackbot [`Subscription`] into a
/// [`Coinbase`] channel to be subscribed to.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct CoinbaseChannel(pub &'static str);

impl CoinbaseChannel {
    /// [`Coinbase`] real-time trades channel.
    ///
    /// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#match>
    pub const TRADES: Self = Self("matches");

    /// [`Coinbase`] level 1 order book channel.
    ///
    /// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#level2>
    pub const BOOK_L1: Self = Self("level2");

    /// [`Coinbase`] level 2 order book channel.
    ///
    /// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#full>
    pub const BOOK_L2: Self = Self("full");
}

impl<Instrument> Identifier<CoinbaseChannel> for Subscription<Coinbase, Instrument, PublicTrades> {
    fn id(&self) -> CoinbaseChannel {
        CoinbaseChannel::TRADES
    }
}

impl<Instrument> Identifier<CoinbaseChannel> for Subscription<Coinbase, Instrument, OrderBooksL2> {
    fn id(&self) -> CoinbaseChannel {
        CoinbaseChannel::BOOK_L2
    }
}

impl AsRef<str> for CoinbaseChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
