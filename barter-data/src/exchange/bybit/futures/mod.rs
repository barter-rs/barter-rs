use barter_integration::model::instrument::Instrument;
use l2::BybitPerpetualsBookUpdater;
use liquidation::BybitLiquidation;

use super::{Bybit, ExchangeServer};
use crate::{
    exchange::{ExchangeId, StreamSelector},
    instrument::InstrumentData,
    subscription::{book::OrderBooksL2, liquidation::Liquidations},
    transformer::{book::MultiBookTransformer, stateless::StatelessTransformer},
    ExchangeWsStream,
};

/// [`BybitPerpetualsUsd`] WebSocket server base url.
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/ws/connect>
pub const WEBSOCKET_BASE_URL_BYBIT_PERPETUALS_USD: &str = "wss://stream.bybit.com/v5/public/linear";

/// Level 2 OrderBook types (top of book) and perpetual
/// [`OrderBookUpdater`](crate::transformer::book::OrderBookUpdater) implementation.
pub mod l2;

/// Liquidation types.
pub mod liquidation;

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

impl<Instrument> StreamSelector<Instrument, Liquidations> for BybitPerpetualsUsd
where
    Instrument: InstrumentData,
{
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Id, Liquidations, BybitLiquidation>,
    >;
}
