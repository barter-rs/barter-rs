use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use fnv::FnvHashMap;
use futures::{Stream, stream::SelectAll};
use futures_util::{StreamExt, future::try_join_all};
use itertools::Itertools;
use tokio_stream::wrappers::UnboundedReceiverStream;
use vecmap::VecMap;

use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    Validator,
    channel::{UnboundedRx, UnboundedTx, mpsc_unbounded},
    error::SocketError,
};

use crate::{
    Identifier,
    error::DataError,
    exchange::{
        binance::{futures::BinanceFuturesUsd, market::BinanceMarket, spot::BinanceSpot},
        bitfinex::{Bitfinex, market::BitfinexMarket},
        bitmex::{Bitmex, market::BitmexMarket},
        bybit::{futures::BybitPerpetualsUsd, market::BybitMarket, spot::BybitSpot},
        coinbase::{Coinbase, market::CoinbaseMarket},
        gateio::{
            future::{GateioFuturesBtc, GateioFuturesUsd},
            market::GateioMarket,
            option::GateioOptions,
            perpetual::{GateioPerpetualsBtc, GateioPerpetualsUsd},
            spot::GateioSpot,
        },
        kraken::{Kraken, market::KrakenMarket},
        okx::{Okx, market::OkxMarket},
    },
    instrument::InstrumentData,
    streams::{
        consumer::{MarketStreamResult, STREAM_RECONNECTION_POLICY, init_market_stream},
        reconnect::stream::ReconnectingStream,
    },
    subscription::{
        SubKind, Subscription,
        book::{OrderBookEvent, OrderBookL1, OrderBooksL1, OrderBooksL2},
        liquidation::{Liquidation, Liquidations},
        trade::{PublicTrade, PublicTrades},
    },
};
pub mod indexed;

macro_rules! init_stream_logic {
    (
        $exchange_struct:expr,
        $sub_kind_struct:ident,
        $txs:expr,
        $exchange_id_for_get:expr,
        $tx_field:ident,
        $subs_vec:ident
    ) => {
        init_market_stream(
            STREAM_RECONNECTION_POLICY,
            $subs_vec
                .into_iter()
                .map(|sub| Subscription::new($exchange_struct, sub.instrument, $sub_kind_struct))
                .collect(),
        )
        .await
        .map(|stream| {
            tokio::spawn(
                stream.forward_to($txs.$tx_field.get(&$exchange_id_for_get).unwrap().clone()),
            )
        })
    };
}

#[derive(Debug)]
struct StreamBatch<InstrumentKey> {
    pub trades:
        VecMap<ExchangeId, UnboundedReceiverStream<MarketStreamResult<InstrumentKey, PublicTrade>>>,
    pub l1s:
        VecMap<ExchangeId, UnboundedReceiverStream<MarketStreamResult<InstrumentKey, OrderBookL1>>>,
    pub l2s: VecMap<
        ExchangeId,
        UnboundedReceiverStream<MarketStreamResult<InstrumentKey, OrderBookEvent>>,
    >,
    pub liquidations:
        VecMap<ExchangeId, UnboundedReceiverStream<MarketStreamResult<InstrumentKey, Liquidation>>>,
}

#[derive(Debug)]
pub struct DynamicStreams<InstrumentKey> {
    pub batches: Vec<StreamBatch<InstrumentKey>>,
}

impl<InstrumentKey> DynamicStreams<InstrumentKey> {
    /// Initialise a set of `Streams` by providing one or more [`Subscription`] batches.
    ///
    /// Each batch (ie/ `impl Iterator<Item = Subscription>`) will initialise at-least-one
    /// WebSocket `Stream` under the hood. If the batch contains more-than-one [`ExchangeId`] and/or
    /// [`SubKind`], it will be further split under the hood for compile-time reasons.
    ///
    /// ## Examples
    /// Please see barter-data-rs/examples/dynamic_multi_stream_multi_exchange.rs for a
    /// comprehensive example of how to use this market data stream initialiser.
    pub async fn init<SubBatchIter, SubIter, Sub, Instrument>(
        subscription_batches: SubBatchIter,
    ) -> Result<Self, DataError>
    where
        SubBatchIter: IntoIterator<Item = SubIter>,
        SubIter: IntoIterator<Item = Sub>,
        Sub: Into<Subscription<ExchangeId, Instrument, SubKind>>,
        Instrument: InstrumentData<Key = InstrumentKey> + Ord + Display + 'static,
        InstrumentKey: Debug + Clone + Send + 'static,
        Subscription<BinanceSpot, Instrument, PublicTrades>: Identifier<BinanceMarket>,
        Subscription<BinanceSpot, Instrument, OrderBooksL1>: Identifier<BinanceMarket>,
        Subscription<BinanceSpot, Instrument, OrderBooksL2>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, PublicTrades>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, OrderBooksL1>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, OrderBooksL2>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, Liquidations>: Identifier<BinanceMarket>,
        Subscription<Bitfinex, Instrument, PublicTrades>: Identifier<BitfinexMarket>,
        Subscription<Bitmex, Instrument, PublicTrades>: Identifier<BitmexMarket>,
        Subscription<BybitSpot, Instrument, PublicTrades>: Identifier<BybitMarket>,
        Subscription<BybitSpot, Instrument, OrderBooksL1>: Identifier<BybitMarket>,
        Subscription<BybitSpot, Instrument, OrderBooksL2>: Identifier<BybitMarket>,
        Subscription<BybitPerpetualsUsd, Instrument, PublicTrades>: Identifier<BybitMarket>,
        Subscription<BybitPerpetualsUsd, Instrument, OrderBooksL1>: Identifier<BybitMarket>,
        Subscription<BybitPerpetualsUsd, Instrument, OrderBooksL2>: Identifier<BybitMarket>,
        Subscription<Coinbase, Instrument, PublicTrades>: Identifier<CoinbaseMarket>,
        Subscription<GateioSpot, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<GateioFuturesUsd, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<GateioFuturesBtc, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<GateioPerpetualsUsd, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<GateioPerpetualsBtc, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<GateioOptions, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<Kraken, Instrument, PublicTrades>: Identifier<KrakenMarket>,
        Subscription<Kraken, Instrument, OrderBooksL1>: Identifier<KrakenMarket>,
        Subscription<Okx, Instrument, PublicTrades>: Identifier<OkxMarket>,
    {
        let futures = subscription_batches
            .into_iter()
            .map(|sub_batch| async move {
                let validated_batch = validate_subscriptions(sub_batch)?;
                Self::init_streams_for_single_batch(validated_batch).await
            });

        let batches = futures::future::try_join_all(futures).await?;

        Ok(Self {
            batches,
        })
    }

    async fn init_streams_for_single_batch<Instrument>(
        mut batch: Vec<Subscription<ExchangeId, Instrument, SubKind>>,
    ) -> Result<StreamBatch<InstrumentKey>, DataError>
    where
        Instrument: InstrumentData<Key = InstrumentKey> + Ord + Display + 'static,
        InstrumentKey: Debug + Clone + Send + 'static,
        Subscription<BinanceSpot, Instrument, PublicTrades>: Identifier<BinanceMarket>,
        Subscription<BinanceSpot, Instrument, OrderBooksL1>: Identifier<BinanceMarket>,
        Subscription<BinanceSpot, Instrument, OrderBooksL2>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, PublicTrades>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, OrderBooksL1>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, OrderBooksL2>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, Liquidations>: Identifier<BinanceMarket>,
        Subscription<Bitfinex, Instrument, PublicTrades>: Identifier<BitfinexMarket>,
        Subscription<Bitmex, Instrument, PublicTrades>: Identifier<BitmexMarket>,
        Subscription<BybitSpot, Instrument, PublicTrades>: Identifier<BybitMarket>,
        Subscription<BybitSpot, Instrument, OrderBooksL1>: Identifier<BybitMarket>,
        Subscription<BybitSpot, Instrument, OrderBooksL2>: Identifier<BybitMarket>,
        Subscription<BybitPerpetualsUsd, Instrument, PublicTrades>: Identifier<BybitMarket>,
        Subscription<BybitPerpetualsUsd, Instrument, OrderBooksL1>: Identifier<BybitMarket>,
        Subscription<BybitPerpetualsUsd, Instrument, OrderBooksL2>: Identifier<BybitMarket>,
        Subscription<Coinbase, Instrument, PublicTrades>: Identifier<CoinbaseMarket>,
        Subscription<GateioSpot, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<GateioFuturesUsd, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<GateioFuturesBtc, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<GateioPerpetualsUsd, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<GateioPerpetualsBtc, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<GateioOptions, Instrument, PublicTrades>: Identifier<GateioMarket>,
        Subscription<Kraken, Instrument, PublicTrades>: Identifier<KrakenMarket>,
        Subscription<Kraken, Instrument, OrderBooksL1>: Identifier<KrakenMarket>,
        Subscription<Okx, Instrument, PublicTrades>: Identifier<OkxMarket>,
    {
        let channels = Channels::try_from(&batch)?;
        batch.sort_unstable_by_key(|sub| (sub.exchange, sub.kind));
        let by_exchange_by_sub_kind = batch.into_iter().chunk_by(|sub| (sub.exchange, sub.kind));

        let batch_futures =
            by_exchange_by_sub_kind
                .into_iter()
                .map(|((exchange, sub_kind), subs)| {
                    let subs = subs.into_iter().collect::<Vec<_>>();
                    let txs = Arc::clone(&channels.txs);
                    async move {
                        match (exchange, sub_kind) {
                            (ExchangeId::BinanceSpot, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    BinanceSpot::default(),
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::BinanceSpot, SubKind::OrderBooksL1) => {
                                init_stream_logic!(
                                    BinanceSpot::default(),
                                    OrderBooksL1,
                                    txs,
                                    exchange,
                                    l1s,
                                    subs
                                )
                            }
                            (ExchangeId::BinanceSpot, SubKind::OrderBooksL2) => {
                                init_stream_logic!(
                                    BinanceSpot::default(),
                                    OrderBooksL2,
                                    txs,
                                    exchange,
                                    l2s,
                                    subs
                                )
                            }
                            (ExchangeId::BinanceFuturesUsd, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    BinanceFuturesUsd::default(),
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::BinanceFuturesUsd, SubKind::OrderBooksL1) => {
                                init_stream_logic!(
                                    BinanceFuturesUsd::default(),
                                    OrderBooksL1,
                                    txs,
                                    exchange,
                                    l1s,
                                    subs
                                )
                            }
                            (ExchangeId::BinanceFuturesUsd, SubKind::OrderBooksL2) => {
                                init_stream_logic!(
                                    BinanceFuturesUsd::default(),
                                    OrderBooksL2,
                                    txs,
                                    exchange,
                                    l2s,
                                    subs
                                )
                            }
                            (ExchangeId::BinanceFuturesUsd, SubKind::Liquidations) => {
                                init_stream_logic!(
                                    BinanceFuturesUsd::default(),
                                    Liquidations,
                                    txs,
                                    exchange,
                                    liquidations,
                                    subs
                                )
                            }
                            (ExchangeId::Bitfinex, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    Bitfinex,
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::Bitmex, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    Bitmex,
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::Coinbase, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    Coinbase,
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::Okx, SubKind::PublicTrades) => {
                                init_stream_logic!(Okx, PublicTrades, txs, exchange, trades, subs)
                            }
                            (ExchangeId::BybitSpot, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    BybitSpot::default(),
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::BybitSpot, SubKind::OrderBooksL1) => {
                                init_stream_logic!(
                                    BybitSpot::default(),
                                    OrderBooksL1,
                                    txs,
                                    exchange,
                                    l1s,
                                    subs
                                )
                            }
                            (ExchangeId::BybitSpot, SubKind::OrderBooksL2) => {
                                init_stream_logic!(
                                    BybitSpot::default(),
                                    OrderBooksL2,
                                    txs,
                                    exchange,
                                    l2s,
                                    subs
                                )
                            }
                            (ExchangeId::BybitPerpetualsUsd, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    BybitPerpetualsUsd::default(),
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::BybitPerpetualsUsd, SubKind::OrderBooksL1) => {
                                init_stream_logic!(
                                    BybitPerpetualsUsd::default(),
                                    OrderBooksL1,
                                    txs,
                                    exchange,
                                    l1s,
                                    subs
                                )
                            }
                            (ExchangeId::BybitPerpetualsUsd, SubKind::OrderBooksL2) => {
                                init_stream_logic!(
                                    BybitPerpetualsUsd::default(),
                                    OrderBooksL2,
                                    txs,
                                    exchange,
                                    l2s,
                                    subs
                                )
                            }
                            (ExchangeId::GateioSpot, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    GateioSpot::default(),
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::GateioFuturesUsd, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    GateioFuturesUsd::default(),
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::GateioFuturesBtc, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    GateioFuturesBtc::default(),
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::GateioPerpetualsUsd, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    GateioPerpetualsUsd::default(),
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::GateioPerpetualsBtc, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    GateioPerpetualsBtc::default(),
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::GateioOptions, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    GateioOptions::default(),
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::Kraken, SubKind::PublicTrades) => {
                                init_stream_logic!(
                                    Kraken,
                                    PublicTrades,
                                    txs,
                                    exchange,
                                    trades,
                                    subs
                                )
                            }
                            (ExchangeId::Kraken, SubKind::OrderBooksL1) => {
                                init_stream_logic!(Kraken, OrderBooksL1, txs, exchange, l1s, subs)
                            }

                            // Fallback for unsupported combinations
                            (exchange, sub_kind) => {
                                // Use different names for clarity
                                Err(DataError::Unsupported { exchange, sub_kind })
                            }
                        }
                    }
                });

        try_join_all(batch_futures).await?;
        Ok(StreamBatch {
            trades: channels
                .rxs
                .trades
                .into_iter()
                .map(|(exchange, rx)| (exchange, rx.into_stream()))
                .collect(),
            l1s: channels
                .rxs
                .l1s
                .into_iter()
                .map(|(exchange, rx)| (exchange, rx.into_stream()))
                .collect(),
            l2s: channels
                .rxs
                .l2s
                .into_iter()
                .map(|(exchange, rx)| (exchange, rx.into_stream()))
                .collect(),
            liquidations: channels
                .rxs
                .liquidations
                .into_iter()
                .map(|(exchange, rx)| (exchange, rx.into_stream()))
                .collect(),
        })
    }

    /// Remove an exchange [`PublicTrade`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_trades(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, PublicTrade>>>>
    {
        let streams = self
            .batches
            .iter_mut()
            .filter_map(|batch| batch.trades.remove(&exchange))
            .collect::<Vec<_>>();
        (!streams.is_empty()).then(|| futures_util::stream::select_all::select_all(streams))
    }

    /// Select and merge every exchange [`PublicTrade`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all::select_all).
    pub fn select_all_trades(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, PublicTrade>>> {
        let streams = self
            .batches
            .iter_mut()
            .flat_map(|batch| std::mem::take(&mut batch.trades).into_values())
            .collect::<Vec<_>>();
        futures_util::stream::select_all::select_all(streams)
    }

    /// Remove an exchange [`OrderBookL1`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_l1s(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, OrderBookL1>>>>
    {
        let streams = self
            .batches
            .iter_mut()
            .filter_map(|batch| batch.l1s.remove(&exchange))
            .collect::<Vec<_>>();

        (!streams.is_empty()).then(|| futures_util::stream::select_all::select_all(streams))
    }

    /// Select and merge every exchange [`OrderBookL1`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all::select_all).
    pub fn select_all_l1s(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, OrderBookL1>>> {
        let streams = self
            .batches
            .iter_mut()
            .flat_map(|batch| std::mem::take(&mut batch.l1s).into_values())
            .collect::<Vec<_>>();
        futures_util::stream::select_all::select_all(streams)
    }

    /// Remove an exchange [`OrderBookEvent`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_l2s(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, OrderBookEvent>>>>
    {
        let streams = self
            .batches
            .iter_mut()
            .filter_map(|batch| batch.l2s.remove(&exchange))
            .collect::<Vec<_>>();

        (!streams.is_empty()).then(|| futures_util::stream::select_all::select_all(streams))
    }

    /// Select and merge every exchange [`OrderBookEvent`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all::select_all).
    pub fn select_all_l2s(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, OrderBookEvent>>> {
        let streams = self
            .batches
            .iter_mut()
            .flat_map(|batch| std::mem::take(&mut batch.l2s).into_values())
            .collect::<Vec<_>>();
        futures_util::stream::select_all::select_all(streams)
    }

    /// Remove an exchange [`Liquidation`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_liquidations(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, Liquidation>>>>
    {
        let streams = self
            .batches
            .iter_mut()
            .filter_map(|batch| batch.liquidations.remove(&exchange))
            .collect::<Vec<_>>();

        (!streams.is_empty()).then(|| futures_util::stream::select_all::select_all(streams))
    }

    /// Select and merge every exchange [`Liquidation`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all::select_all).
    pub fn select_all_liquidations(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, Liquidation>>> {
        let streams = self
            .batches
            .iter_mut()
            .flat_map(|batch| std::mem::take(&mut batch.liquidations).into_values())
            .collect::<Vec<_>>();
        futures_util::stream::select_all::select_all(streams)
    }

    /// Select and merge every exchange `Stream` for every data type using [`select_all`](futures_util::stream::select_all::select_all)
    ///
    /// Note that using [`MarketEvent<Instrument, DataKind>`] as the `Output` is suitable for most
    /// use cases.
    pub fn select_all<Output>(self) -> impl Stream<Item = Output>
    where
        InstrumentKey: Send + 'static,
        Output: 'static,
        MarketStreamResult<InstrumentKey, PublicTrade>: Into<Output>,
        MarketStreamResult<InstrumentKey, OrderBookL1>: Into<Output>,
        MarketStreamResult<InstrumentKey, OrderBookEvent>: Into<Output>,
        MarketStreamResult<InstrumentKey, Liquidation>: Into<Output>,
    {
        let mut all_trades = Vec::new();
        let mut all_l1s = Vec::new();
        let mut all_l2s = Vec::new();
        let mut all_liquidations = Vec::new();

        for mut batch in self.batches {
            all_trades.extend(std::mem::take(&mut batch.trades).into_values());
            all_l1s.extend(std::mem::take(&mut batch.l1s).into_values());
            all_l2s.extend(std::mem::take(&mut batch.l2s).into_values());
            all_liquidations.extend(std::mem::take(&mut batch.liquidations).into_values());
        }

        let trades = all_trades
            .into_iter()
            .map(|stream| stream.map(MarketStreamResult::into).boxed());

        let l1s = all_l1s
            .into_iter()
            .map(|stream| stream.map(MarketStreamResult::into).boxed());

        let l2s = all_l2s
            .into_iter()
            .map(|stream| stream.map(MarketStreamResult::into).boxed());

        let liquidations = all_liquidations
            .into_iter()
            .map(|stream| stream.map(MarketStreamResult::into).boxed());

        let all = trades.chain(l1s).chain(l2s).chain(liquidations);

        futures_util::stream::select_all::select_all(all)
    }
}

pub fn validate_subscriptions<SubIter, Sub, Instrument>(
    batch: SubIter,
) -> Result<Vec<Subscription<ExchangeId, Instrument, SubKind>>, DataError>
where
    SubIter: IntoIterator<Item = Sub>,
    Sub: Into<Subscription<ExchangeId, Instrument, SubKind>>,
    Instrument: InstrumentData + Ord,
{
    // Validate Subscriptions
    let mut batch = batch
        .into_iter()
        .map(Sub::into)
        .map(Validator::validate)
        .collect::<Result<Vec<_>, SocketError>>()?;

    // Remove duplicate Subscriptions
    batch.sort();
    batch.dedup();

    Ok(batch)
}

struct Channels<InstrumentKey> {
    txs: Arc<Txs<InstrumentKey>>,
    rxs: Rxs<InstrumentKey>,
}

impl<'a, Instrument> TryFrom<&'a Vec<Subscription<ExchangeId, Instrument, SubKind>>>
    for Channels<Instrument::Key>
where
    Instrument: InstrumentData,
{
    type Error = DataError;

    fn try_from(
        value: &'a Vec<Subscription<ExchangeId, Instrument, SubKind>>,
    ) -> Result<Self, Self::Error> {
        let mut txs = Txs::default();
        let mut rxs = Rxs::default();

        for sub in value {
            match sub.kind {
                SubKind::PublicTrades => {
                    if let (None, None) =
                        (txs.trades.get(&sub.exchange), rxs.trades.get(&sub.exchange))
                    {
                        let (tx, rx) = mpsc_unbounded();
                        txs.trades.insert(sub.exchange, tx);
                        rxs.trades.insert(sub.exchange, rx);
                    }
                }
                SubKind::OrderBooksL1 => {
                    if let (None, None) = (txs.l1s.get(&sub.exchange), rxs.l1s.get(&sub.exchange)) {
                        let (tx, rx) = mpsc_unbounded();
                        txs.l1s.insert(sub.exchange, tx);
                        rxs.l1s.insert(sub.exchange, rx);
                    }
                }
                SubKind::OrderBooksL2 => {
                    if let (None, None) =
                        (txs.l2s.get(&sub.exchange), rxs.l2s.get(&sub.exchange))
                    {
                        let (tx, rx) = mpsc_unbounded();
                        txs.l2s.insert(sub.exchange, tx);
                        rxs.l2s.insert(sub.exchange, rx);
                    }
                }
                SubKind::Liquidations => {
                    if let (None, None) = (
                        txs.liquidations.get(&sub.exchange),
                        rxs.liquidations.get(&sub.exchange),
                    ) {
                        let (tx, rx) = mpsc_unbounded();
                        txs.liquidations.insert(sub.exchange, tx);
                        rxs.liquidations.insert(sub.exchange, rx);
                    }
                }
                unsupported => return Err(DataError::UnsupportedSubKind(unsupported)),
            }
        }

        Ok(Channels {
            txs: Arc::new(txs),
            rxs,
        })
    }
}

struct Txs<InstrumentKey> {
    trades: FnvHashMap<ExchangeId, UnboundedTx<MarketStreamResult<InstrumentKey, PublicTrade>>>,
    l1s: FnvHashMap<ExchangeId, UnboundedTx<MarketStreamResult<InstrumentKey, OrderBookL1>>>,
    l2s: FnvHashMap<ExchangeId, UnboundedTx<MarketStreamResult<InstrumentKey, OrderBookEvent>>>,
    liquidations:
        FnvHashMap<ExchangeId, UnboundedTx<MarketStreamResult<InstrumentKey, Liquidation>>>,
}

impl<InstrumentKey> Default for Txs<InstrumentKey> {
    fn default() -> Self {
        Self {
            trades: Default::default(),
            l1s: Default::default(),
            l2s: Default::default(),
            liquidations: Default::default(),
        }
    }
}

struct Rxs<InstrumentKey> {
    trades: FnvHashMap<ExchangeId, UnboundedRx<MarketStreamResult<InstrumentKey, PublicTrade>>>,
    l1s: FnvHashMap<ExchangeId, UnboundedRx<MarketStreamResult<InstrumentKey, OrderBookL1>>>,
    l2s: FnvHashMap<ExchangeId, UnboundedRx<MarketStreamResult<InstrumentKey, OrderBookEvent>>>,
    liquidations:
        FnvHashMap<ExchangeId, UnboundedRx<MarketStreamResult<InstrumentKey, Liquidation>>>,
}

impl<InstrumentKey> Default for Rxs<InstrumentKey> {
    fn default() -> Self {
        Self {
            trades: Default::default(),
            l1s: Default::default(),
            l2s: Default::default(),
            liquidations: Default::default(),
        }
    }
}
