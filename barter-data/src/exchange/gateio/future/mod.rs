use crate::{
    Identifier, NoInitialSnapshots, StreamSelector,
    error::DataError,
    event::MarketEvent,
    exchange::{
        ExchangeServer,
        gateio::{
            Gateio, channel::GateioChannel, market::GateioMarket,
            perpetual::trade::GateioFuturesTrades,
        },
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
    Subscription<Self, Instrument, PublicTrades>:
        Identifier<GateioChannel> + Identifier<GateioMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, PublicTrades>>> + Send + Send,
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

impl Display for GateioFuturesBtc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioFuturesBtc")
    }
}
