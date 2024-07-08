use self::l2::BinanceSpotBookUpdater;
use super::{Binance, ExchangeServer};
use crate::{
    exchange::{ExchangeId, StreamSelector},
    subscription::book::OrderBooksL2,
    transformer::book::MultiBookTransformer,
    ExchangeWsStream,
};
use barter_integration::model::instrument::Instrument;

/// Level 2 OrderBook types (top of book) and spot
/// [`OrderBookUpdater`](crate::transformer::book::OrderBookUpdater) implementation.
pub mod l2;

/// [`BinanceSpot`] WebSocket server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams>
pub const WEBSOCKET_BASE_URL_BINANCE_SPOT: &str = "wss://stream.binance.com:9443/ws";

/// [`Binance`] spot exchange.
pub type BinanceSpot = Binance<BinanceServerSpot>;

/// [`Binance`] spot [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct BinanceServerSpot;

impl ExchangeServer for BinanceServerSpot {
    const ID: ExchangeId = ExchangeId::BinanceSpot;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_BINANCE_SPOT
    }
}

impl StreamSelector<Instrument, OrderBooksL2> for BinanceSpot {
    type Stream = ExchangeWsStream<
        MultiBookTransformer<Self, Instrument, OrderBooksL2, BinanceSpotBookUpdater>,
    >;
}
