use super::CoinbaseInternational;
use crate::{
    subscription::{trade::PublicTrades, Subscription},
    Identifier,
};
use serde::Serialize;
use crate::subscription::book::{OrderBooksL1, OrderBooksL2};
use crate::subscription::candle::Candles;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`CoinbaseInternational`] channel to be subscribed to.
///
/// See docs: <https://docs.cdp.coinbase.com/advanced-trade/docs/ws-channels>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct CoinbaseInternationalChannel(pub &'static str);

impl CoinbaseInternationalChannel {
    /// Real-time server pings to keep all connections open.
    pub const HEARTBEATS: Self = Self("heartbeats");

    /// Real-time updates on product candles.
    pub const CANDLES: Self = Self("candles");

    /// Sends all products and currencies on a preset interval.
    pub const STATUS: Self = Self("status");

    /// Real-time price updates every time a match happens.
    pub const TICKER: Self = Self("ticker");

    /// Real-time price updates every 5000 milli-seconds.
    pub const TICKER_BATCH: Self = Self("ticker_batch");

    /// All updates and easiest way to keep order book snapshot.
    pub const LEVEL2: Self = Self("level2");

    /// Only sends messages that include the authenticated user.
    pub const USER: Self = Self("user");

    /// Real-time updates every time a market trade happens.
    pub const MARKET_TRADES: Self = Self("market_trades");

    /// Real-time updates every time a user's futures balance changes.
    pub const FUTURES_BALANCE_SUMMARY: Self = Self("futures_balance_summary");
}

impl AsRef<str> for CoinbaseInternationalChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<Instrument> Identifier<CoinbaseInternationalChannel> for Subscription<CoinbaseInternational, Instrument, PublicTrades> {
    fn id(&self) -> CoinbaseInternationalChannel {
        CoinbaseInternationalChannel::MARKET_TRADES
    }
}
impl<Instrument> Identifier<CoinbaseInternationalChannel> for Subscription<CoinbaseInternational, Instrument, OrderBooksL1> {
    fn id(&self) -> CoinbaseInternationalChannel {
        CoinbaseInternationalChannel::TICKER
    }
}
impl<Instrument> Identifier<CoinbaseInternationalChannel> for Subscription<CoinbaseInternational, Instrument, OrderBooksL2> {
    fn id(&self) -> CoinbaseInternationalChannel {
        CoinbaseInternationalChannel::LEVEL2
    }
}

impl<Instrument> Identifier<CoinbaseInternationalChannel> for Subscription<CoinbaseInternational, Instrument, Candles> {
    fn id(&self) -> CoinbaseInternationalChannel {
        CoinbaseInternationalChannel::CANDLES
    }
}
