use crate::{
    Identifier, LiveMarketDataArgs, NoInitialSnapshots,
    error::DataError,
    event::MarketEvent,
    exchange::{
        ExchangeServer,
        gateio::{Gateio, market::GateioMarket, perpetual::trade::GateioFuturesTrades},
    },
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

impl<Instrument> DataStream<LiveMarketDataArgs<Self, Instrument, PublicTrades>> for GateioOptions
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
            StatelessTransformer<Self, Instrument::Key, PublicTrades, GateioFuturesTrades>,
            NoInitialSnapshots,
        >(args)
        .await
    }
}

impl Display for GateioOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioOptions")
    }
}
