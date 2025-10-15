use self::trade::GateioFuturesTrades;
use super::Gateio;
use crate::{
    NoInitialSnapshots,
    exchange::{ExchangeServer, StreamSelector, gateio::GateiotWsStream},
    instrument::InstrumentData,
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use std::fmt::Display;

/// Public trades types.
pub mod trade;

/// [`GateioPerpetualsUsd`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_PERPETUALS_USD: &str = "wss://fx-ws.gateio.ws/v4/ws/usdt";

/// [`Gateio`] perpetual usd exchange.
pub type GateioPerpetualsUsd = Gateio<GateioServerPerpetualsUsd>;

/// [`Gateio`] perpetual usd [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct GateioServerPerpetualsUsd;

impl ExchangeServer for GateioServerPerpetualsUsd {
    const ID: ExchangeId = ExchangeId::GateioPerpetualsUsd;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_PERPETUALS_USD
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for GateioPerpetualsUsd
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = GateiotWsStream<
        StatelessTransformer<Self, Instrument::Key, PublicTrades, GateioFuturesTrades>,
    >;
}

impl Display for GateioPerpetualsUsd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioPerpetualsUsd")
    }
}

/// [`GateioPerpetualsBtc`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_PERPETUALS_BTC: &str = "wss://fx-ws.gateio.ws/v4/ws/btc";

/// [`Gateio`] perpetual btc exchange.
pub type GateioPerpetualsBtc = Gateio<GateioServerPerpetualsBtc>;

/// [`Gateio`] perpetual btc [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct GateioServerPerpetualsBtc;

impl ExchangeServer for GateioServerPerpetualsBtc {
    const ID: ExchangeId = ExchangeId::GateioPerpetualsBtc;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_PERPETUALS_BTC
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for GateioPerpetualsBtc
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = GateiotWsStream<
        StatelessTransformer<Self, Instrument::Key, PublicTrades, GateioFuturesTrades>,
    >;
}

impl Display for GateioPerpetualsBtc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioPerpetualsBtc")
    }
}
