use fnv::FnvHashMap;
use futures::Stream;
use futures_util::future::try_join_all;
use futures_util::{FutureExt, StreamExt};
use itertools::Itertools;
use barter_integration::error::SocketError;
use barter_integration::Validator;
use crate::error::DataError;
use crate::exchange::binance::channel::BinanceChannel;
use crate::exchange::binance::market::BinanceMarket;
use crate::exchange::binance::spot::BinanceSpot;
use crate::exchange::{ExchangeId, StreamSelector};
use crate::exchange::binance::futures::BinanceFuturesUsd;
use crate::exchange::bitfinex::Bitfinex;
use crate::exchange::bitfinex::channel::BitfinexChannel;
use crate::exchange::bitfinex::market::BitfinexMarket;
use crate::exchange::bitmex::Bitmex;
use crate::exchange::bitmex::channel::BitmexChannel;
use crate::exchange::bitmex::market::BitmexMarket;
use crate::exchange::bybit::channel::BybitChannel;
use crate::exchange::bybit::futures::BybitPerpetualsUsd;
use crate::exchange::bybit::market::BybitMarket;
use crate::exchange::bybit::spot::BybitSpot;
use crate::exchange::coinbase::channel::CoinbaseChannel;
use crate::exchange::coinbase::Coinbase;
use crate::exchange::coinbase::market::CoinbaseMarket;
use crate::exchange::gateio::channel::GateioChannel;
use crate::exchange::gateio::future::{GateioFuturesBtc, GateioFuturesUsd};
use crate::exchange::gateio::market::GateioMarket;
use crate::exchange::gateio::option::GateioOptions;
use crate::exchange::gateio::perpetual::{GateioPerpetualsBtc, GateioPerpetualsUsd};
use crate::exchange::gateio::spot::GateioSpot;
use crate::exchange::kraken::channel::KrakenChannel;
use crate::exchange::kraken::Kraken;
use crate::exchange::kraken::market::KrakenMarket;
use crate::exchange::okx::channel::OkxChannel;
use crate::exchange::okx::market::OkxMarket;
use crate::exchange::okx::Okx;
use crate::Identifier;
use crate::instrument::InstrumentData;
use crate::streams::consumer::{init_market_stream, MarketStreamEvent, STREAM_RECONNECTION_POLICY};
use crate::subscription::{Subscription, SubscriptionKind};
use crate::subscription::book::{OrderBookEvent};
use crate::subscription::liquidation::{Liquidation};
use crate::subscription::trade::{PublicTrade};

pub struct MarketStreamManager<StTrades, StL1s, StL2s, StLiqs> {
    pub trades: FnvHashMap<ExchangeId, StTrades>,
    pub l1s: FnvHashMap<ExchangeId, StL1s>,
    pub l2s: FnvHashMap<ExchangeId, StL2s>,
    pub liquidations: FnvHashMap<ExchangeId, StLiqs>,
}

// Todo: could limit MarketStreamBuilder to a single Kind at a time... or even a single ws at a time
//  '--> using enums is very useful though maybe just enum for ExchangeId?

impl<StTrades, StL1s, StL2s, StLiqs> MarketStreamManager<StTrades, StL1s, StL2s, StLiqs> {
    pub fn select_trades<InstrumentKey>(
        &mut self,
        key: &ExchangeId
    ) -> Option<StTrades>
    where
        StTrades: Stream<Item = MarketStreamEvent<InstrumentKey, PublicTrade>>
    {
        self.trades.remove(key)
    }

    pub fn select_l1s<InstrumentKey>(
        &mut self,
        key: &ExchangeId
    ) -> Option<StL1s>
    where
        StL1s: Stream<Item = MarketStreamEvent<InstrumentKey, PublicTrade>>
    {
        self.l1s.remove(key)
    }

    pub fn select_l2s<InstrumentKey>(
        &mut self,
        key: &ExchangeId
    ) -> Option<StL2s>
    where
        StL2s: Stream<Item = MarketStreamEvent<InstrumentKey, OrderBookEvent>>
    {
        self.l2s.remove(key)
    }

    pub fn select_liquidations<InstrumentKey>(
        &mut self,
        key: &ExchangeId
    ) -> Option<StLiqs>
    where
        StLiqs: Stream<Item = MarketStreamEvent<InstrumentKey, Liquidation>>
    {
        self.liquidations.remove(key)
    }
}

pub async fn subscribe<SubIter, Sub, Instrument, Kind>(
    subscriptions: SubIter,
) -> Result<FnvHashMap<ExchangeId, impl Stream>, DataError>
where
    SubIter: IntoIterator<Item = Sub>,
    Sub: Into<Subscription<ExchangeId, Instrument, Kind>>,
    Instrument: InstrumentData + Ord + 'static,
    Kind: SubscriptionKind + Clone + Ord + Send + 'static,
    Kind::Event: Send,
    Subscription<ExchangeId, Instrument, Kind>: Validator,
    BinanceSpot: StreamSelector<Instrument, Kind>,
    BinanceFuturesUsd: StreamSelector<Instrument, Kind>,
    Bitfinex: StreamSelector<Instrument, Kind>,
    Bitmex: StreamSelector<Instrument, Kind>,
    BybitSpot: StreamSelector<Instrument, Kind>,
    BybitPerpetualsUsd: StreamSelector<Instrument, Kind>,
    Coinbase: StreamSelector<Instrument, Kind>,
    GateioSpot: StreamSelector<Instrument, Kind>,
    GateioFuturesUsd: StreamSelector<Instrument, Kind>,
    GateioFuturesBtc: StreamSelector<Instrument, Kind>,
    GateioPerpetualsUsd: StreamSelector<Instrument, Kind>,
    GateioPerpetualsBtc: StreamSelector<Instrument, Kind>,
    GateioOptions: StreamSelector<Instrument, Kind>,
    Kraken: StreamSelector<Instrument, Kind>,
    Okx: StreamSelector<Instrument, Kind>,
    Subscription<BinanceSpot, Instrument, Kind>: Identifier<BinanceChannel> + Identifier<BinanceMarket>,
    Subscription<BinanceFuturesUsd, Instrument, Kind>: Identifier<BinanceChannel> + Identifier<BinanceMarket>,
    Subscription<Bitfinex, Instrument, Kind>: Identifier<BitfinexChannel> + Identifier<BitfinexMarket>,
    Subscription<Bitmex, Instrument, Kind>: Identifier<BitmexChannel> + Identifier<BitmexMarket>,
    Subscription<BybitSpot, Instrument, Kind>: Identifier<BybitChannel> + Identifier<BybitMarket>,
    Subscription<BybitPerpetualsUsd, Instrument, Kind>: Identifier<BybitChannel> + Identifier<BybitMarket>,
    Subscription<Coinbase, Instrument, Kind>: Identifier<CoinbaseChannel> + Identifier<CoinbaseMarket>,
    Subscription<GateioSpot, Instrument, Kind>: Identifier<GateioChannel> + Identifier<GateioMarket>,
    Subscription<GateioFuturesUsd, Instrument, Kind>: Identifier<GateioChannel> + Identifier<GateioMarket>,
    Subscription<GateioFuturesBtc, Instrument, Kind>: Identifier<GateioChannel> + Identifier<GateioMarket>,
    Subscription<GateioPerpetualsUsd, Instrument, Kind>: Identifier<GateioChannel> + Identifier<GateioMarket>,
    Subscription<GateioPerpetualsBtc, Instrument, Kind>: Identifier<GateioChannel> + Identifier<GateioMarket>,
    Subscription<GateioOptions, Instrument, Kind>: Identifier<GateioChannel> + Identifier<GateioMarket>,
    Subscription<Kraken, Instrument, Kind>: Identifier<KrakenChannel> + Identifier<KrakenMarket>,
    Subscription<Okx, Instrument, Kind>: Identifier<OkxChannel> + Identifier<OkxMarket>,
{
    // Validate & dedup Subscriptions
    let mut subscriptions = validate_subscriptions(subscriptions)?;

    // Chunk by ExchangeId
    subscriptions.sort_unstable_by_key(|sub| sub.exchange);

    let by_exchange = subscriptions
        .into_iter()
        .chunk_by(|sub| sub.exchange);

    let futures = by_exchange
        .into_iter()
        .map(|(exchange, subs)| async move {
            crate::match_exchange!(exchange; Exchange => {
                init_market_stream::<Exchange, Instrument, Kind>(
                    STREAM_RECONNECTION_POLICY,
                    subs.into_iter()
                        .map(|sub| Subscription::new(
                            Exchange::default(),
                            sub.instrument,
                            sub.kind
                        ))
                        .collect()
                ).await.map(|stream| (exchange, stream.boxed()))
            })
        });

    try_join_all(futures)
        .await
        .map(|streams| streams.into_iter().collect())
}

pub fn validate_subscriptions<SubIter, Sub, Instrument, Kind>(
    batch: SubIter,
) -> Result<Vec<Subscription<ExchangeId, Instrument, Kind>>, DataError>
where
    SubIter: IntoIterator<Item = Sub>,
    Sub: Into<Subscription<ExchangeId, Instrument, Kind>>,
    Instrument: InstrumentData + Ord,
    Kind: Ord,
    Subscription<ExchangeId, Instrument, Kind>: Validator
{
    // Validate Subscriptions
    let mut batch = batch
        .into_iter()
        .map(Sub::into)
        .map(Subscription::validate)
        .collect::<Result<Vec<_>, SocketError>>()?;

    // Remove duplicate Subscriptions
    batch.sort();
    batch.dedup();

    Ok(batch)
}



#[macro_export]
macro_rules! match_exchange {
    ($exchange_variant:expr; $exchange_ty:ident => $block:block) => {
        match ($exchange_variant) {
            ExchangeId::BinanceSpot => {
                type $exchange_ty = crate::exchange::binance::spot::BinanceSpot;
                $block
            },
            ExchangeId::BinanceFuturesUsd => {
                type $exchange_ty = crate::exchange::binance::futures::BinanceFuturesUsd;
                $block
            },
            ExchangeId::Bitfinex => {
                type $exchange_ty = crate::exchange::bitfinex::Bitfinex;
                $block
            },
            ExchangeId::Bitmex => {
                type $exchange_ty = crate::exchange::bitmex::Bitmex;
                $block
            },
            ExchangeId::BybitSpot => {
                type $exchange_ty = crate::exchange::bybit::spot::BybitSpot;
                $block
            },
            ExchangeId::BybitPerpetualsUsd => {
                type $exchange_ty = crate::exchange::bybit::futures::BybitPerpetualsUsd;
                $block
            },
            ExchangeId::Coinbase => {
                type $exchange_ty = crate::exchange::coinbase::Coinbase;
                $block
            },
            ExchangeId::GateioSpot => {
                type $exchange_ty = crate::exchange::gateio::spot::GateioSpot;
                $block
            },
            ExchangeId::GateioFuturesUsd => {
                type $exchange_ty = crate::exchange::gateio::future::GateioFuturesUsd;
                $block
            },
            ExchangeId::GateioFuturesBtc => {
                type $exchange_ty = crate::exchange::gateio::future::GateioFuturesBtc;
                $block
            },
            ExchangeId::GateioPerpetualsBtc => {
                type $exchange_ty = crate::exchange::gateio::perpetual::GateioPerpetualsBtc;
                $block
            },
            ExchangeId::GateioPerpetualsUsd => {
                type $exchange_ty = crate::exchange::gateio::perpetual::GateioPerpetualsUsd;
                $block
            },
            ExchangeId::GateioOptions => {
                type $exchange_ty = crate::exchange::gateio::option::GateioOptions;
                $block
            },
            ExchangeId::Kraken => {
                type $exchange_ty = crate::exchange::kraken::Kraken;
                $block
            },
            ExchangeId::Okx => {
                type $exchange_ty = crate::exchange::okx::Okx;
                $block
            },
        }
    };
}

#[macro_export]
macro_rules! match_sub_kind {
    ($sub_kind_variant:expr; $sub_kind_ty:ident => $block:block) => {{
        match $sub_kind_variant {
            SubKind::PublicTrades => {
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            SubKind::OrderBooksL1 => {
                type $sub_kind_ty = crate::subscription::book::OrderBooksL1;
                $block
            },
            SubKind::OrderBooksL2 => {
                type $sub_kind_ty = crate::subscription::book::OrderBooksL2;
                $block
            },
            SubKind::OrderBooksL3 => {
                type $sub_kind_ty = crate::subscription::book::OrderBooksL3;
                $block
            },
            SubKind::Liquidations => {
                type $sub_kind_ty = crate::subscription::liquidation::Liquidations;
                $block
            },
            SubKind::Candles => {
                type $sub_kind_ty = crate::subscription::candle::Candles;
                $block
            }
        }
    }};
}

#[macro_export]
macro_rules! match_exchange_sub_kind {
    ($exchange_variant:expr; $sub_kind_variant:expr; $exchange_ty:ident; $sub_kind_ty:ident => $block:block) => {{
        match ($exchange_variant, $sub_kind_variant) {
            (ExchangeId::BinanceSpot, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::binance::spot::BinanceSpot;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::BinanceSpot, SubKind::OrderBooksL1) => {
                type $exchange_ty = crate::exchange::binance::spot::BinanceSpot;
                type $sub_kind_ty = crate::subscription::book::OrderBooksL1;
                $block
            },
            (ExchangeId::BinanceFuturesUsd, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::binance::futures::BinanceFuturesUsd;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::BinanceFuturesUsd, SubKind::OrderBooksL1) => {
                type $exchange_ty = crate::exchange::binance::futures::BinanceFuturesUsd;
                type $sub_kind_ty = crate::subscription::book::OrderBooksL1;
                $block
            },
            (ExchangeId::BinanceFuturesUsd, SubKind::Liquidations) => {
                type $exchange_ty = crate::exchange::binance::futures::BinanceFuturesUsd;
                type $sub_kind_ty = crate::subscription::liquidation::Liquidations;
                $block
            },
            (ExchangeId::Bitfinex, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::bitfinex::Bitfinex;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::Bitmex, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::bitmex::Bitmex;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::BybitSpot, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::bybit::spot::BybitSpot;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::BybitPerpetualsUsd, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::bybit::futures::BybitPerpetualsUsd;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::Coinbase, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::coinbase::Coinbase;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::GateioSpot, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::gateio::spot::GateioSpot;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::GateioFuturesUsd, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::gateio::future::GateioFuturesUsd;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::GateioFuturesBtc, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::gateio::future::GateioFuturesBtc;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::GateioPerpetualsBtc, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::gateio::perpetual::GateioPerpetualsBtc;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::GateioPerpetualsUsd, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::gateio::perpetual::GateioPerpetualsUsd;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::GateioOptions, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::gateio::option::GateioOptions;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::Kraken, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::kraken::Kraken;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            (ExchangeId::Kraken, SubKind::OrderBooksL1) => {
                type $exchange_ty = crate::exchange::kraken::Kraken;
                type $sub_kind_ty = crate::subscription::book::OrderBooksL1;
                $block
            },
            (ExchangeId::Okx, SubKind::PublicTrades) => {
                type $exchange_ty = crate::exchange::bitfinex::Bitfinex;
                type $sub_kind_ty = crate::subscription::trade::PublicTrades;
                $block
            },
            _ => {
                panic!("unsupported ExchangeId, SubKind combination");
            }
        }
    }};
}
