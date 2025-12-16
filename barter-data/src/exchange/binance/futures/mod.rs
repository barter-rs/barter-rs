use self::liquidation::BinanceLiquidation;
use super::{Binance, ExchangeServer};
use crate::{
    Identifier, NoInitialSnapshots, StreamSelector,
    error::DataError,
    event::MarketEvent,
    exchange::binance::{
        channel::BinanceChannel,
        futures::l2::{
            BinanceFuturesUsdOrderBooksL2SnapshotFetcher, BinanceFuturesUsdOrderBooksL2Transformer,
        },
        market::BinanceMarket,
    },
    init_ws_exchange_stream_with_initial_snapshots,
    instrument::InstrumentData,
    subscription::{Subscription, SubscriptionKind, book::OrderBooksL2, liquidation::Liquidations},
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::protocol::websocket::WebSocketSerdeParser;
use futures::Stream;
use std::{
    fmt::{Display, Formatter},
    future::Future,
};

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

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for BinanceFuturesUsd
where
    Instrument: InstrumentData,
    Subscription<Self, Instrument, OrderBooksL2>:
        Identifier<BinanceChannel> + Identifier<BinanceMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, OrderBooksL2>>>,
    ) -> impl Future<
        Output = Result<
            impl Stream<
                Item = Result<
                    MarketEvent<Instrument::Key, <OrderBooksL2 as SubscriptionKind>::Event>,
                    DataError,
                >,
            >,
            DataError,
        >,
    > {
        init_ws_exchange_stream_with_initial_snapshots::<
            Self,
            Instrument,
            OrderBooksL2,
            WebSocketSerdeParser,
            BinanceFuturesUsdOrderBooksL2Transformer<Instrument::Key>,
            BinanceFuturesUsdOrderBooksL2SnapshotFetcher,
        >(subscriptions)
    }
}

impl<Instrument> StreamSelector<Instrument, Liquidations> for BinanceFuturesUsd
where
    Instrument: InstrumentData,
    Subscription<Self, Instrument, Liquidations>:
        Identifier<BinanceChannel> + Identifier<BinanceMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, Liquidations>>>,
    ) -> impl Future<
        Output = Result<
            impl Stream<
                Item = Result<
                    MarketEvent<Instrument::Key, <Liquidations as SubscriptionKind>::Event>,
                    DataError,
                >,
            >,
            DataError,
        >,
    > {
        init_ws_exchange_stream_with_initial_snapshots::<
            Self,
            Instrument,
            Liquidations,
            WebSocketSerdeParser,
            StatelessTransformer<Self, Instrument::Key, Liquidations, BinanceLiquidation>,
            NoInitialSnapshots,
        >(subscriptions)
    }
}

impl Display for BinanceFuturesUsd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BinanceFuturesUsd")
    }
}
