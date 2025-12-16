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

/// [`GateioOptions`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/futures/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_OPTIONS_USD: &str = "wss://op-ws.gateio.live/v4/ws";

/// [`Gateio`] options exchange.
pub type GateioOptions = Gateio<GateioServerOptions>;

/// [`Gateio`] options [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct GateioServerOptions;

impl ExchangeServer for GateioServerOptions {
    const ID: ExchangeId = ExchangeId::GateioOptions;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_OPTIONS_USD
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for GateioOptions
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

impl Display for GateioOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioOptions")
    }
}
