use barter_integration::model::instrument::Instrument;
use l2::BybitPerpetualsBookUpdater;

use super::{Bybit, ExchangeServer};
use crate::{
    exchange::{ExchangeId, StreamSelector},
    subscription::book::OrderBooksL2,
    transformer::book::MultiBookTransformer,
    ExchangeWsStream,
};

/// [`BybitPerpetualsUsd`] WebSocket server base url.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
pub const WEBSOCKET_BASE_URL_BYBIT_PERPETUALS_USD: &str = "wss://stream.bybit.com/v5/public/linear";

/// Level 2 OrderBook types (top of book) and perpetual
/// [`OrderBookUpdater`](crate::transformer::book::OrderBookUpdater) implementation.
pub mod l2;

/// [`Bybit`] perpetual exchange.
pub type BybitPerpetualsUsd = Bybit<BybitServerPerpetualsUsd>;

/// [`Bybit`] perpetual [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct BybitServerPerpetualsUsd;

impl ExchangeServer for BybitServerPerpetualsUsd {
    const ID: ExchangeId = ExchangeId::BybitPerpetualsUsd;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_BYBIT_PERPETUALS_USD
    }
}

impl StreamSelector<Instrument, OrderBooksL2> for BybitPerpetualsUsd {
    type Stream = ExchangeWsStream<
        MultiBookTransformer<Self, Instrument, OrderBooksL2, BybitPerpetualsBookUpdater>,
    >;
}
