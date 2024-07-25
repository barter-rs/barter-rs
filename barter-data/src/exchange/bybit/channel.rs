use crate::{
    exchange::bybit::Bybit,
    subscription::{
        book::{OrderBooksL1, OrderBooksL2}, liquidation::Liquidations, trade::PublicTrades, Subscription
    },
    Identifier,
};
use serde::Serialize;

use super::futures::BybitPerpetualsUsd;

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
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook>
    pub const ORDER_BOOK_L1: Self = Self("orderbook.1");
    pub const ORDER_BOOK_L2: Self = Self("orderbook.50");
    /// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/liquidation>
    pub const LIQUIDATIONS: Self = Self("liquidation");
}

impl<Server, Instrument> Identifier<BybitChannel>
    for Subscription<Bybit<Server>, Instrument, PublicTrades>
{
    fn id(&self) -> BybitChannel {
        BybitChannel::TRADES
    }
}

impl<Server, Instrument> Identifier<BybitChannel>
    for Subscription<Bybit<Server>, Instrument, OrderBooksL1>
{
    fn id(&self) -> BybitChannel {
        BybitChannel::ORDER_BOOK_L1
    }
}

impl<Server, Instrument> Identifier<BybitChannel>
    for Subscription<Bybit<Server>, Instrument, OrderBooksL2>
{
    fn id(&self) -> BybitChannel {
        BybitChannel::ORDER_BOOK_L2
    }
}

impl<Instrument> Identifier<BybitChannel>
    for Subscription<BybitPerpetualsUsd, Instrument, Liquidations>
{
    fn id(&self) -> BybitChannel {
        BybitChannel::LIQUIDATIONS
    }
}

impl AsRef<str> for BybitChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
