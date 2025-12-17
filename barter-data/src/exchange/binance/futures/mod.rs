use self::liquidation::BinanceLiquidation;
use super::{Binance, ExchangeServer};
use crate::{
    Identifier, LiveMarketDataArgs, NoInitialSnapshots,
    error::DataError,
    event::MarketEvent,
    exchange::binance::{
        channel::BinanceChannel,
        futures::l2::{
            BinanceFuturesUsdOrderBooksL2SnapshotFetcher, BinanceFuturesUsdOrderBooksL2Transformer,
        },
        market::BinanceMarket,
    },
    init_ws_exchange_stream,
    instrument::InstrumentData,
    subscription::{
        Subscription,
        book::{OrderBookEvent, OrderBooksL2},
        liquidation::{Liquidation, Liquidations},
    },
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{serde::de::DeJson, stream::data::DataStream};
use futures::Stream;
use std::fmt::{Display, Formatter};

/// Level 2 OrderBook types.
pub mod l2;

/// Liquidation types.
pub mod liquidation;

/// [`BinanceFuturesUsd`] WebSocket server base url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#websocket-market-streams>
pub const WEBSOCKET_BASE_URL_BINANCE_FUTURES_USD: &str = "wss://fstream.binance.com/ws";

/// [`Binance`] perpetual usd exchange.
pub type BinanceFuturesUsd = Binance<BinanceServerFuturesUsd>;

/// [`Binance`] perpetual usd [`ExchangeServer`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct BinanceServerFuturesUsd;

impl ExchangeServer for BinanceServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::BinanceFuturesUsd;

    fn websocket_url() -> &'static str {
        WEBSOCKET_BASE_URL_BINANCE_FUTURES_USD
    }
}

impl<Instrument> DataStream<LiveMarketDataArgs<Self, Instrument, OrderBooksL2>>
    for BinanceFuturesUsd
where
    Instrument: InstrumentData + 'static,
    Subscription<Self, Instrument, OrderBooksL2>:
        Identifier<BinanceChannel> + Identifier<BinanceMarket>,
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
            BinanceFuturesUsdOrderBooksL2Transformer<Instrument::Key>,
            BinanceFuturesUsdOrderBooksL2SnapshotFetcher,
        >(args)
        .await
    }
}

impl<Instrument> DataStream<LiveMarketDataArgs<Self, Instrument, Liquidations>>
    for BinanceFuturesUsd
where
    Instrument: InstrumentData + 'static,
    Subscription<Self, Instrument, Liquidations>:
        Identifier<BinanceChannel> + Identifier<BinanceMarket>,
{
    type Item = Result<MarketEvent<Instrument::Key, Liquidation>, DataError>;
    type Error = DataError;

    async fn init(
        args: LiveMarketDataArgs<Self, Instrument, Liquidations>,
    ) -> Result<impl Stream<Item = Self::Item>, Self::Error> {
        init_ws_exchange_stream::<
            Self,
            Instrument,
            Liquidations,
            DeJson,
            StatelessTransformer<Self, Instrument::Key, Liquidations, BinanceLiquidation>,
            NoInitialSnapshots,
        >(args)
        .await
    }
}

impl Display for BinanceFuturesUsd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BinanceFuturesUsd")
    }
}
