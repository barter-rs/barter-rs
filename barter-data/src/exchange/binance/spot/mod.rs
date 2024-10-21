use super::{Binance, ExchangeServer};
use crate::{
    exchange::{
        binance::spot::l2::{
            BinanceSpotOrderBooksL2SnapshotFetcher, BinanceSpotOrderBooksL2Transformer,
        },
        ExchangeId, StreamSelector,
    },
    instrument::InstrumentData,
    subscription::book::OrderBooksL2,
    ExchangeWsStream,
};

/// Level 2 OrderBook types.
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

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for BinanceSpot
where
    Instrument: InstrumentData,
{
    type SnapFetcher = BinanceSpotOrderBooksL2SnapshotFetcher;
    type Stream = ExchangeWsStream<BinanceSpotOrderBooksL2Transformer<Instrument::Key>>;
}
