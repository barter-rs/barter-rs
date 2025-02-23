use super::Bitfinex;
use crate::{
    Identifier,
    subscription::{Subscription, trade::PublicTrades},
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Bitfinex`] channel to be subscribed to.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-public>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BitfinexChannel(pub &'static str);

impl BitfinexChannel {
    /// [`Bitfinex`] real-time trades channel.
    ///
    /// See docs: <https://docs.bitfinex.com/reference/ws-public-trades>
    pub const TRADES: Self = Self("trades");
}

impl<Instrument> Identifier<BitfinexChannel> for Subscription<Bitfinex, Instrument, PublicTrades> {
    fn id(&self) -> BitfinexChannel {
        BitfinexChannel::TRADES
    }
}

impl AsRef<str> for BitfinexChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
