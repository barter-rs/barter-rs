use self::trade::GateioFuturesTrades;
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

impl<Instrument> DataStream<LiveMarketDataArgs<Self, Instrument, PublicTrades>>
    for GateioPerpetualsUsd
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

impl<Instrument> DataStream<LiveMarketDataArgs<Self, Instrument, PublicTrades>>
    for GateioPerpetualsBtc
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

impl Display for GateioPerpetualsBtc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GateioPerpetualsBtc")
    }
}
