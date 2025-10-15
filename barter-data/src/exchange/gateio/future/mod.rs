use crate::{
    NoInitialSnapshots,
    exchange::{
        ExchangeServer, StreamSelector,
        gateio::{Gateio, GateiotWsStream, perpetual::trade::GateioFuturesTrades},
    },
    instrument::InstrumentData,
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use std::fmt::Display;

/// [`GateioFuturesUsd`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/delivery/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_FUTURES_USD: &str = "wss://fx-ws.gateio.ws/v4/ws/delivery/usdt";

/// [`Gateio`] perpetual usd exchange.
pub type GateioFuturesUsd = Gateio<GateioServerFuturesUsd>;

/// [`Gateio`] perpetual usd [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct GateioServerFuturesUsd;

impl ExchangeServer for GateioServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::GateioFuturesUsd;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_FUTURES_USD
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for GateioFuturesUsd
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = GateiotWsStream<
        StatelessTransformer<Self, Instrument::Key, PublicTrades, GateioFuturesTrades>,
    >;
}

impl Display for GateioFuturesUsd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioFuturesUsd")
    }
}

/// [`GateioFuturesBtc`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/delivery/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_FUTURES_BTC: &str = "wss://fx-ws.gateio.ws/v4/ws/delivery/btc";

/// [`Gateio`] perpetual btc exchange.
pub type GateioFuturesBtc = Gateio<GateioServerFuturesBtc>;

/// [`Gateio`] perpetual btc [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct GateioServerFuturesBtc;

impl ExchangeServer for GateioServerFuturesBtc {
    const ID: ExchangeId = ExchangeId::GateioFuturesBtc;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_FUTURES_BTC
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for GateioFuturesBtc
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = GateiotWsStream<
        StatelessTransformer<Self, Instrument::Key, PublicTrades, GateioFuturesTrades>,
    >;
}

impl Display for GateioFuturesBtc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioFuturesBtc")
    }
}
