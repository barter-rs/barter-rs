use super::Ibkr;
use crate::{
    subscription::{book::OrderBooksL1, Subscription},
    Identifier,
};
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Ibkr`](super::Ibkr) channel to be subscribed to.
///
/// See docs: TODO: update to IBKR <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct IbkrChannel {
    pub slug: &'static str,
    pub sub_type: &'static str,
}

impl IbkrChannel {
    /// [`Ibkr`] real-time market data (order book L1) channel.
    ///
    /// See docs: TODO: update to IBKR <https://www.okx.com/docs-v5/en/#websocket-api-public-channel-trades-channel>
    pub const ORDER_BOOK_L1: Self = Self {
        slug: "OrderBookL1",
        sub_type: "md"
    };
}

impl<Instrument> Identifier<IbkrChannel> for Subscription<Ibkr, Instrument, OrderBooksL1> {
    fn id(&self) -> IbkrChannel {
        IbkrChannel::ORDER_BOOK_L1
    }
}

impl AsRef<str> for IbkrChannel {
    fn as_ref(&self) -> &str {
        &self.sub_type
    }
}
