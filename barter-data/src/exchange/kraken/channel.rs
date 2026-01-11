use super::KrakenSpot;
use crate::{
    impl_channel_identifier,
    subscription::{book::OrderBooksL1, trade::PublicTrades},
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Kraken`](super::spot::KrakenSpot) channel to be subscribed to.
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct KrakenChannel(pub &'static str);

impl KrakenChannel {
    /// [`Kraken`](super::spot::KrakenSpot) real-time trades channel name.
    ///
    /// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
    pub const TRADES: Self = Self("trade");

    /// [`Kraken`](super::spot::KrakenSpot) real-time OrderBook Level1 (top of books) channel name.
    ///
    /// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
    pub const ORDER_BOOK_L1: Self = Self("spread");
}

impl_channel_identifier!(KrakenSpot, Instrument => KrakenChannel, PublicTrades => KrakenChannel::TRADES);
impl_channel_identifier!(KrakenSpot, Instrument => KrakenChannel, OrderBooksL1 => KrakenChannel::ORDER_BOOK_L1);

impl AsRef<str> for KrakenChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kraken_channel_serialize_trades() {
        let channel = KrakenChannel::TRADES;
        assert_eq!(channel.as_ref(), "trade");
    }

    #[test]
    fn test_kraken_channel_serialize_l1() {
        let channel = KrakenChannel::ORDER_BOOK_L1;
        assert_eq!(channel.as_ref(), "spread");
    }
}
