use self::liquidation::BinanceLiquidation;
use super::{Binance, ExchangeServer};
use crate::exchange::binance::futures::l2::BinanceFuturesUsdOrderBooksL2Transformer;
use crate::subscription::book::OrderBooksL2;
use crate::{
    exchange::{ExchangeId, StreamSelector},
    instrument::InstrumentData,
    subscription::liquidation::Liquidations,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream,
};

/// Level 2 OrderBook types (top of books) and perpetual
/// [`OrderBookUpdater`](crate::transformer::book::OrderBookUpdater) implementation.
pub mod l2;

/// Liquidation types.
pub mod liquidation;

/// [`BinanceFuturesUsd`] WebSocket server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
pub const WEBSOCKET_BASE_URL_BINANCE_FUTURES_USD: &str = "wss://fstream.binance.com/ws";

/// [`Binance`] perpetual usd exchange.
pub type BinanceFuturesUsd = Binance<BinanceServerFuturesUsd>;

/// [`Binance`] perpetual usd [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct BinanceServerFuturesUsd;

impl ExchangeServer for BinanceServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::BinanceFuturesUsd;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_BINANCE_FUTURES_USD
    }
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for BinanceFuturesUsd
where
    Instrument: InstrumentData,
{
    type Stream = ExchangeWsStream<BinanceFuturesUsdOrderBooksL2Transformer<Instrument::Key>>;
}

impl<Instrument> StreamSelector<Instrument, Liquidations> for BinanceFuturesUsd
where
    Instrument: InstrumentData,
{
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, Liquidations, BinanceLiquidation>,
    >;
}
