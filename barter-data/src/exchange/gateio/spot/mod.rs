use self::trade::GateioSpotTrade;
use super::Gateio;
use crate::{
    Identifier, LiveMarketDataArgs, NoInitialSnapshots,
    error::DataError,
    event::MarketEvent,
    exchange::{ExchangeServer, gateio::market::GateioMarket},
    init_ws_exchange_stream,
    instrument::InstrumentData,
    subscription::{
        Subscription,
        trade::{PublicTrade, PublicTrades},
    },
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{serde::de::DeJson, stream::data::DataStream};
use futures_util::Stream;
use std::fmt::Display;

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

impl<Instrument> DataStream<LiveMarketDataArgs<Self, Instrument, PublicTrades>> for GateioSpot
where
    Instrument: InstrumentData + 'static,
    Subscription<Self, Instrument, PublicTrades>: Identifier<GateioMarket>,
{
    type Item = Result<MarketEvent<Instrument::Key, PublicTrade>, DataError>;
    type Error = DataError;

    async fn init(
        args: LiveMarketDataArgs<Self, Instrument, PublicTrades>,
    ) -> Result<impl Stream<Item = Self::Item>, Self::Error> {
        init_ws_exchange_stream::<
            Self,
            Instrument,
            PublicTrades,
            DeJson,
            StatelessTransformer<Self, Instrument::Key, PublicTrades, GateioSpotTrade>,
            NoInitialSnapshots,
        >(args)
        .await
    }
}

impl Display for GateioSpot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioSpot")
    }
}
