use crate::{
    exchange::bybit::Bybit,
    impl_channel_identifier,
    subscription::{
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bybit`]
/// channel to be subscribed to.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BybitChannel(pub &'static str);

impl BybitChannel {
    /// [`Bybit`] real-time trades channel name.
    ///
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
    pub const TRADES: Self = Self("publicTrade");

    /// [`Bybit`] real-time OrderBook Level1 (top of books) channel name.
    ///
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook>
    pub const ORDER_BOOK_L1: Self = Self("orderbook.1");

    /// [`Bybit`] OrderBook Level2 channel name (20ms delta updates).
    ///
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook>
    pub const ORDER_BOOK_L2: Self = Self("orderbook.50");
}

impl_channel_identifier!(Bybit<Server>, Instrument => BybitChannel, PublicTrades => BybitChannel::TRADES);
impl_channel_identifier!(Bybit<Server>, Instrument => BybitChannel, OrderBooksL1 => BybitChannel::ORDER_BOOK_L1);
impl_channel_identifier!(Bybit<Server>, Instrument => BybitChannel, OrderBooksL2 => BybitChannel::ORDER_BOOK_L2);

impl AsRef<str> for BybitChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
