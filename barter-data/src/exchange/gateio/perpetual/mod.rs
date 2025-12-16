use self::trade::GateioFuturesTrades;
use super::Gateio;
use crate::{
    Identifier, NoInitialSnapshots, StreamSelector,
    error::DataError,
    event::MarketEvent,
    exchange::{
        ExchangeServer,
        gateio::{channel::GateioChannel, market::GateioMarket},
    },
    init_ws_exchange_stream_with_initial_snapshots,
    instrument::InstrumentData,
    subscription::{Subscription, SubscriptionKind, trade::PublicTrades},
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::protocol::websocket::WebSocketSerdeParser;
use futures_util::Stream;
use std::{fmt::Display, future::Future};

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
    Subscription<Self, Instrument, PublicTrades>:
        Identifier<GateioChannel> + Identifier<GateioMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, PublicTrades>>> + Send,
    ) -> impl Future<
        Output = Result<
            impl Stream<
                Item = Result<
                    MarketEvent<Instrument::Key, <PublicTrades as SubscriptionKind>::Event>,
                    DataError,
                >,
            >,
            DataError,
        >,
    > {
        init_ws_exchange_stream_with_initial_snapshots::<
            Self,
            Instrument,
            PublicTrades,
            WebSocketSerdeParser,
            StatelessTransformer<Self, Instrument::Key, PublicTrades, GateioFuturesTrades>,
            NoInitialSnapshots,
        >(subscriptions)
    }
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
    Subscription<Self, Instrument, PublicTrades>:
        Identifier<GateioChannel> + Identifier<GateioMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, PublicTrades>>> + Send,
    ) -> impl Future<
        Output = Result<
            impl Stream<
                Item = Result<
                    MarketEvent<Instrument::Key, <PublicTrades as SubscriptionKind>::Event>,
                    DataError,
                >,
            >,
            DataError,
        >,
    > {
        init_ws_exchange_stream_with_initial_snapshots::<
            Self,
            Instrument,
            PublicTrades,
            WebSocketSerdeParser,
            StatelessTransformer<Self, Instrument::Key, PublicTrades, GateioFuturesTrades>,
            NoInitialSnapshots,
        >(subscriptions)
    }
}

impl Display for GateioPerpetualsBtc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioPerpetualsBtc")
    }
}
