use super::{Binance, ExchangeServer};
use crate::{
    Identifier, LiveMarketDataArgs,
    error::DataError,
    event::MarketEvent,
    exchange::binance::{
        market::BinanceMarket,
        spot::l2::{BinanceSpotOrderBooksL2SnapshotFetcher, BinanceSpotOrderBooksL2Transformer},
    },
    init_ws_exchange_stream,
    instrument::InstrumentData,
    subscription::{
        Subscription,
        book::{OrderBookEvent, OrderBooksL2},
    },
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{serde::de::DeJson, stream::data::DataStream};
use futures::Stream;
use std::fmt::{Display, Formatter};

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

impl<Instrument> DataStream<LiveMarketDataArgs<Self, Instrument, OrderBooksL2>> for BinanceSpot
where
    Instrument: InstrumentData + 'static,
    Subscription<Self, Instrument, OrderBooksL2>: Identifier<BinanceMarket>,
{
    type Item = Result<MarketEvent<Instrument::Key, OrderBookEvent>, DataError>;
    type Error = DataError;

    async fn init(
        args: LiveMarketDataArgs<Self, Instrument, OrderBooksL2>,
    ) -> Result<impl Stream<Item = Self::Item>, Self::Error> {
        init_ws_exchange_stream::<
            Self,
            Instrument,
            OrderBooksL2,
            DeJson,
            BinanceSpotOrderBooksL2Transformer<Instrument::Key>,
            BinanceSpotOrderBooksL2SnapshotFetcher,
        >(args)
        .await
    }
}

impl Display for BinanceSpot {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BinanceSpot")
    }
}
