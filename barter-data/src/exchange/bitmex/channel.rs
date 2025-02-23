use crate::{
    Identifier,
    exchange::bitmex::Bitmex,
    subscription::{Subscription, trade::PublicTrades},
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a [`Bitmex`]
/// channel to be subscribed to.
///
/// See docs: <https://www.bitmex.com/app/wsAPI>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BitmexChannel(pub &'static str);

impl BitmexChannel {
    /// [`Bitmex`] real-time trades channel name.
    ///
    /// See docs: <https://www.bitmex.com/app/wsAPI>
    pub const TRADES: Self = Self("trade");
}

impl<Instrument> Identifier<BitmexChannel> for Subscription<Bitmex, Instrument, PublicTrades> {
    fn id(&self) -> BitmexChannel {
        BitmexChannel::TRADES
    }
}

impl AsRef<str> for BitmexChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
