use super::{Binance, futures::BinanceFuturesUsd};
use crate::{
    Identifier,
    subscription::{
        Subscription,
        book::{OrderBooksL1, OrderBooksL2},
        liquidation::Liquidations,
        trade::PublicTrades,
    },
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a [`Binance`]
/// channel to be subscribed to.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams>
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BinanceChannel(pub &'static str);

impl BinanceChannel {
    /// [`Binance`] real-time trades channel name.
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#trade-streams>
    ///
    /// Note:
    /// For [`BinanceFuturesUsd`] this real-time
    /// stream is undocumented.
    ///
    /// See discord: <https://discord.com/channels/910237311332151317/923160222711812126/975712874582388757>
    pub const TRADES: Self = Self("@trade");

    /// [`Binance`] real-time OrderBook Level1 (top of books) channel name.
    ///
    /// See docs:<https://binance-docs.github.io/apidocs/spot/en/#individual-symbol-book-ticker-streams>
    /// See docs:<https://binance-docs.github.io/apidocs/futures/en/#individual-symbol-book-ticker-streams>
    pub const ORDER_BOOK_L1: Self = Self("@bookTicker");

    /// [`Binance`] OrderBook Level2 channel name (100ms delta updates).
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#diff-depth-stream>
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#diff-book-depth-streams>
    pub const ORDER_BOOK_L2: Self = Self("@depth@100ms");

    /// [`BinanceFuturesUsd`] liquidation orders channel name.
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#liquidation-order-streams>
    pub const LIQUIDATIONS: Self = Self("@forceOrder");
}

impl<Server, Instrument> Identifier<BinanceChannel>
    for Subscription<Binance<Server>, Instrument, PublicTrades>
{
    fn id(&self) -> BinanceChannel {
        BinanceChannel::TRADES
    }
}

impl<Server, Instrument> Identifier<BinanceChannel>
    for Subscription<Binance<Server>, Instrument, OrderBooksL1>
{
    fn id(&self) -> BinanceChannel {
        BinanceChannel::ORDER_BOOK_L1
    }
}

impl<Server, Instrument> Identifier<BinanceChannel>
    for Subscription<Binance<Server>, Instrument, OrderBooksL2>
{
    fn id(&self) -> BinanceChannel {
        BinanceChannel::ORDER_BOOK_L2
    }
}

impl<Instrument> Identifier<BinanceChannel>
    for Subscription<BinanceFuturesUsd, Instrument, Liquidations>
{
    fn id(&self) -> BinanceChannel {
        BinanceChannel::LIQUIDATIONS
    }
}

impl AsRef<str> for BinanceChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
