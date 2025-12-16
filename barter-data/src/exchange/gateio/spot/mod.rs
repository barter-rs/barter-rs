use self::trade::GateioSpotTrade;
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

/// [`GateioSpot`] WebSocket server base url.
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/>
pub const WEBSOCKET_BASE_URL_GATEIO_SPOT: &str = "wss://api.gateio.ws/ws/v4/";

/// [`Gateio`] spot exchange.
pub type GateioSpot = Gateio<GateioServerSpot>;

/// [`Gateio`] spot [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct GateioServerSpot;

impl ExchangeServer for GateioServerSpot {
    const ID: ExchangeId = ExchangeId::GateioSpot;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_GATEIO_SPOT
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for GateioSpot
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
            StatelessTransformer<Self, Instrument::Key, PublicTrades, GateioSpotTrade>,
            NoInitialSnapshots,
        >(subscriptions)
    }
}

impl Display for GateioSpot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioSpot")
    }
}
