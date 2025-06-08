use super::Mexc;
use crate::{
    Identifier,
    subscription::{Subscription, book::OrderBooksL1, trade::PublicTrades},
};
use serde::Serialize;

/// Defines how to translate a Barter [`Subscription`] into an [`MexcChannel`]
/// base string for WebSocket subscriptions.
///
/// The actual subscription topic sent to MEXC for aggregated book ticker will be
/// dynamically constructed by appending "@<interval>@<symbol>" to this base channel string.
/// For example: "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT".
///
/// Important: This channel uses Protocol Buffers (.pb) for data format.
///
/// MEXC WebSocket API (Spot V3) Documentation:
/// - Individual symbol book ticker: <https://mexcdevelop.github.io/apidocs/spot_v3_en/#individual-symbol-book-ticker-streams>
/// - Public Subscription Method: <https://mexcdevelop.github.io/apidocs/spot_v3_en/#public-subscription>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct MexcChannel(pub &'static str);

impl MexcChannel {
    /// Base channel string for [`Mexc`]'s real-time public aggregated book ticker
    /// stream using Protocol Buffers.
    ///
    /// The specific aggregation interval (e.g., "100ms") and market symbol
    /// (e.g., "BTCUSDT") will be appended to this string (prefixed with "@")
    /// when forming the actual subscription message.
    ///
    /// Example base string: "spot@public.aggre.bookTicker.v3.api.pb"
    pub const AGGREGATED_BOOK_TICKER_PB: Self = Self("spot@public.aggre.bookTicker.v3.api.pb");
    /// Base channel string for [`Mexc`]'s aggregated deals stream.
    ///
    /// Used for [`PublicTrades`] subscriptions.
    pub const AGGREGATED_DEALS_PB: Self = Self("spot@public.aggre.deals.v3.api.pb");
}

impl<Instrument> Identifier<MexcChannel> for Subscription<Mexc, Instrument, PublicTrades> {
    fn id(&self) -> MexcChannel {
        // Use the aggregated deals stream for public trades.
        MexcChannel::AGGREGATED_DEALS_PB
    }
}

impl<Instrument> Identifier<MexcChannel> for Subscription<Mexc, Instrument, OrderBooksL1> {
    fn id(&self) -> MexcChannel {
        MexcChannel::AGGREGATED_BOOK_TICKER_PB
    }
}

impl AsRef<str> for MexcChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
