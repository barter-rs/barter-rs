use super::KrakenSpot;
use crate::{
    impl_channel_identifier,
    subscription::{book::{OrderBooksL1, OrderBooksL2}, trade::PublicTrades},
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Kraken`](super::spot::KrakenSpot) channel to be subscribed to.
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub enum KrakenChannel {
    /// [`Kraken`](super::spot::KrakenSpot) real-time trades channel.
    Trades,
    /// [`Kraken`](super::spot::KrakenSpot) real-time OrderBook Level1 (top of books) channel.
    OrderBookL1,
    /// [`Kraken`](super::spot::KrakenSpot) real-time OrderBook Level2 (full depth) channel.
    OrderBookL2,
}

impl_channel_identifier!(KrakenSpot, Instrument => KrakenChannel, PublicTrades => KrakenChannel::Trades);
impl_channel_identifier!(KrakenSpot, Instrument => KrakenChannel, OrderBooksL1 => KrakenChannel::OrderBookL1);
impl_channel_identifier!(KrakenSpot, Instrument => KrakenChannel, OrderBooksL2 => KrakenChannel::OrderBookL2);

impl AsRef<str> for KrakenChannel {
    fn as_ref(&self) -> &str {
        match self {
            KrakenChannel::Trades => "trade",
            KrakenChannel::OrderBookL1 => "spread",
            KrakenChannel::OrderBookL2 => "book",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kraken_channel_serialize_trades() {
        let channel = KrakenChannel::Trades;
        assert_eq!(channel.as_ref(), "trade");
    }

    #[test]
    fn test_kraken_channel_serialize_l1() {
        let channel = KrakenChannel::OrderBookL1;
        assert_eq!(channel.as_ref(), "spread");
    }
}
