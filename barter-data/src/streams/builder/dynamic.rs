use crate::{
    error::DataError,
    event::MarketEvent,
    exchange::{
        binance::{futures::BinanceFuturesUsd, market::BinanceMarket, spot::BinanceSpot},
        bitfinex::{market::BitfinexMarket, Bitfinex},
        bitmex::{market::BitmexMarket, Bitmex},
        bybit::{futures::BybitPerpetualsUsd, market::BybitMarket, spot::BybitSpot},
        coinbase::{market::CoinbaseMarket, Coinbase},
        gateio::{
            future::{GateioFuturesBtc, GateioFuturesUsd},
            market::GateioMarket,
            option::GateioOptions,
            perpetual::{GateioPerpetualsBtc, GateioPerpetualsUsd},
            spot::GateioSpot,
        },
        kraken::{market::KrakenMarket, Kraken},
        okx::{market::OkxMarket, Okx},
        ExchangeId,
    },
    instrument::InstrumentData,
    streams::{builder::ExchangeChannel, consumer::consume},
    subscription::{
        book::{OrderBook, OrderBookL1, OrderBooksL1},
        liquidation::{Liquidation, Liquidations},
        trade::{PublicTrade, PublicTrades},
        SubKind, Subscription,
    },
    Identifier,
};
use barter_integration::{error::SocketError, Validator};
use futures::{
    stream::{select_all, SelectAll},
    Stream, StreamExt,
};
use itertools::Itertools;
use std::collections::HashMap;
use tokio_stream::wrappers::UnboundedReceiverStream;
use vecmap::VecMap;

#[derive(Debug)]
pub struct DynamicStreams<InstrumentId> {
    pub trades: VecMap<ExchangeId, UnboundedReceiverStream<MarketEvent<InstrumentId, PublicTrade>>>,
    pub l1s: VecMap<ExchangeId, UnboundedReceiverStream<MarketEvent<InstrumentId, OrderBookL1>>>,
    pub l2s: VecMap<ExchangeId, UnboundedReceiverStream<MarketEvent<InstrumentId, OrderBook>>>,
    pub liquidations:
        VecMap<ExchangeId, UnboundedReceiverStream<MarketEvent<InstrumentId, Liquidation>>>,
}

impl<InstrumentId> DynamicStreams<InstrumentId> {
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
        Instrument: InstrumentData<Id = InstrumentId> + Ord + 'static,
        InstrumentId: Clone + Send,
        Subscription<BinanceSpot, Instrument, PublicTrades>: Identifier<BinanceMarket>,
        Subscription<BinanceSpot, Instrument, PublicTrades>: Identifier<BinanceMarket>,
        Subscription<BinanceSpot, Instrument, OrderBooksL1>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, PublicTrades>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, OrderBooksL1>: Identifier<BinanceMarket>,
        Subscription<BinanceFuturesUsd, Instrument, Liquidations>: Identifier<BinanceMarket>,
        Subscription<Bitfinex, Instrument, PublicTrades>: Identifier<BitfinexMarket>,
        Subscription<Bitmex, Instrument, PublicTrades>: Identifier<BitmexMarket>,
        Subscription<BybitSpot, Instrument, PublicTrades>: Identifier<BybitMarket>,
        Subscription<BybitPerpetualsUsd, Instrument, PublicTrades>: Identifier<BybitMarket>,
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
        // Validate & dedup Subscription batches
        let batches = validate_batches(subscription_batches)?;

        let mut channels = Channels::<Instrument::Id>::default();

        for mut batch in batches {
            batch.sort_unstable_by_key(|sub| (sub.exchange, sub.kind));
            let by_exchange_by_sub_kind =
                batch.into_iter().chunk_by(|sub| (sub.exchange, sub.kind));

            for ((exchange, sub_kind), subs) in by_exchange_by_sub_kind.into_iter() {
                match (exchange, sub_kind) {
                    (ExchangeId::BinanceSpot, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<BinanceSpot, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        BinanceSpot::default(),
                                        sub.instrument,
                                        PublicTrades,
                                    )
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::BinanceSpot, SubKind::OrderBooksL1) => {
                        tokio::spawn(consume::<BinanceSpot, Instrument, OrderBooksL1>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        BinanceSpot::default(),
                                        sub.instrument,
                                        OrderBooksL1,
                                    )
                                })
                                .collect(),
                            channels.l1s.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::BinanceFuturesUsd, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<BinanceFuturesUsd, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        BinanceFuturesUsd::default(),
                                        sub.instrument,
                                        PublicTrades,
                                    )
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::BinanceFuturesUsd, SubKind::OrderBooksL1) => {
                        tokio::spawn(consume::<BinanceFuturesUsd, Instrument, OrderBooksL1>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::<_, Instrument, _>::new(
                                        BinanceFuturesUsd::default(),
                                        sub.instrument,
                                        OrderBooksL1,
                                    )
                                })
                                .collect(),
                            channels.l1s.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::BinanceFuturesUsd, SubKind::Liquidations) => {
                        tokio::spawn(consume::<BinanceFuturesUsd, Instrument, Liquidations>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::<_, Instrument, _>::new(
                                        BinanceFuturesUsd::default(),
                                        sub.instrument,
                                        Liquidations,
                                    )
                                })
                                .collect(),
                            channels
                                .liquidations
                                .entry(exchange)
                                .or_default()
                                .tx
                                .clone(),
                        ));
                    }
                    (ExchangeId::Bitfinex, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<Bitfinex, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(Bitfinex, sub.instrument, PublicTrades)
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::Bitmex, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<Bitmex, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| Subscription::new(Bitmex, sub.instrument, PublicTrades))
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::BybitSpot, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<BybitSpot, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        BybitSpot::default(),
                                        sub.instrument,
                                        PublicTrades,
                                    )
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::BybitPerpetualsUsd, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<BybitPerpetualsUsd, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        BybitPerpetualsUsd::default(),
                                        sub.instrument,
                                        PublicTrades,
                                    )
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::Coinbase, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<Coinbase, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(Coinbase, sub.instrument, PublicTrades)
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::GateioSpot, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<GateioSpot, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioSpot::default(),
                                        sub.instrument,
                                        PublicTrades,
                                    )
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::GateioFuturesUsd, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<GateioFuturesUsd, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioFuturesUsd::default(),
                                        sub.instrument,
                                        PublicTrades,
                                    )
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::GateioFuturesBtc, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<GateioFuturesBtc, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioFuturesBtc::default(),
                                        sub.instrument,
                                        PublicTrades,
                                    )
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::GateioPerpetualsUsd, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<GateioPerpetualsUsd, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioPerpetualsUsd::default(),
                                        sub.instrument,
                                        PublicTrades,
                                    )
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::GateioPerpetualsBtc, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<GateioPerpetualsBtc, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioPerpetualsBtc::default(),
                                        sub.instrument,
                                        PublicTrades,
                                    )
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::GateioOptions, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<GateioOptions, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioOptions::default(),
                                        sub.instrument,
                                        PublicTrades,
                                    )
                                })
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::Kraken, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<Kraken, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| Subscription::new(Kraken, sub.instrument, PublicTrades))
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::Kraken, SubKind::OrderBooksL1) => {
                        tokio::spawn(consume::<Kraken, Instrument, OrderBooksL1>(
                            subs.into_iter()
                                .map(|sub| Subscription::new(Kraken, sub.instrument, OrderBooksL1))
                                .collect(),
                            channels.l1s.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (ExchangeId::Okx, SubKind::PublicTrades) => {
                        tokio::spawn(consume::<Okx, Instrument, PublicTrades>(
                            subs.into_iter()
                                .map(|sub| Subscription::new(Okx, sub.instrument, PublicTrades))
                                .collect(),
                            channels.trades.entry(exchange).or_default().tx.clone(),
                        ));
                    }
                    (exchange, sub_kind) => {
                        return Err(DataError::Unsupported { exchange, sub_kind })
                    }
                }
            }
        }

        Ok(Self {
            trades: channels
                .trades
                .into_iter()
                .map(|(exchange, channel)| (exchange, UnboundedReceiverStream::new(channel.rx)))
                .collect(),
            l1s: channels
                .l1s
                .into_iter()
                .map(|(exchange, channel)| (exchange, UnboundedReceiverStream::new(channel.rx)))
                .collect(),
            l2s: channels
                .l2s
                .into_iter()
                .map(|(exchange, channel)| (exchange, UnboundedReceiverStream::new(channel.rx)))
                .collect(),
            liquidations: channels
                .liquidations
                .into_iter()
                .map(|(exchange, channel)| (exchange, UnboundedReceiverStream::new(channel.rx)))
                .collect(),
        })
    }

    /// Remove an exchange [`PublicTrade`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_trades(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<UnboundedReceiverStream<MarketEvent<InstrumentId, PublicTrade>>> {
        self.trades.remove(&exchange)
    }

    /// Select and merge every exchange [`PublicTrade`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all).
    pub fn select_all_trades(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketEvent<InstrumentId, PublicTrade>>> {
        select_all(std::mem::take(&mut self.trades).into_values())
    }

    /// Remove an exchange [`OrderBookL1`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_l1s(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<UnboundedReceiverStream<MarketEvent<InstrumentId, OrderBookL1>>> {
        self.l1s.remove(&exchange)
    }

    /// Select and merge every exchange [`OrderBookL1`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all).
    pub fn select_all_l1s(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketEvent<InstrumentId, OrderBookL1>>> {
        select_all(std::mem::take(&mut self.l1s).into_values())
    }

    /// Remove an exchange [`OrderBook`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_l2s(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<UnboundedReceiverStream<MarketEvent<InstrumentId, OrderBook>>> {
        self.l2s.remove(&exchange)
    }

    /// Select and merge every exchange [`OrderBook`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all).
    pub fn select_all_l2s(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketEvent<InstrumentId, OrderBook>>> {
        select_all(std::mem::take(&mut self.l2s).into_values())
    }

    /// Remove an exchange [`Liquidation`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_liquidations(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<UnboundedReceiverStream<MarketEvent<InstrumentId, Liquidation>>> {
        self.liquidations.remove(&exchange)
    }

    /// Select and merge every exchange [`Liquidation`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all).
    pub fn select_all_liquidations(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketEvent<InstrumentId, Liquidation>>> {
        select_all(std::mem::take(&mut self.liquidations).into_values())
    }

    /// Select and merge every exchange `Stream` for every data type using
    /// [`SelectAll`](futures_util::stream::select_all).
    ///
    /// Note that using [`MarketEvent<Instrument, DataKind>`] as the `Output` is suitable for most
    /// use cases.
    pub fn select_all<Output>(self) -> impl Stream<Item = Output>
    where
        InstrumentId: Send + 'static,
        Output: 'static,
        MarketEvent<InstrumentId, PublicTrade>: Into<Output>,
        MarketEvent<InstrumentId, OrderBookL1>: Into<Output>,
        MarketEvent<InstrumentId, OrderBook>: Into<Output>,
        MarketEvent<InstrumentId, Liquidation>: Into<Output>,
    {
        let Self {
            trades,
            l1s,
            l2s,
            liquidations,
        } = self;

        let trades = trades
            .into_values()
            .map(|stream| stream.map(MarketEvent::into).boxed());

        let l1s = l1s
            .into_values()
            .map(|stream| stream.map(MarketEvent::into).boxed());

        let l2s = l2s
            .into_values()
            .map(|stream| stream.map(MarketEvent::into).boxed());

        let liquidations = liquidations
            .into_values()
            .map(|stream| stream.map(MarketEvent::into).boxed());

        let all = trades.chain(l1s).chain(l2s).chain(liquidations);

        select_all(all)
    }
}

pub fn validate_batches<SubBatchIter, SubIter, Sub, Instrument>(
    batches: SubBatchIter,
) -> Result<Vec<Vec<Subscription<ExchangeId, Instrument, SubKind>>>, DataError>
where
    SubBatchIter: IntoIterator<Item = SubIter>,
    SubIter: IntoIterator<Item = Sub>,
    Sub: Into<Subscription<ExchangeId, Instrument, SubKind>>,
    Instrument: InstrumentData + Ord,
{
    batches
        .into_iter()
        .map(|batch| {
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
        })
        .collect()
}

struct Channels<InstrumentId> {
    trades: HashMap<ExchangeId, ExchangeChannel<MarketEvent<InstrumentId, PublicTrade>>>,
    l1s: HashMap<ExchangeId, ExchangeChannel<MarketEvent<InstrumentId, OrderBookL1>>>,
    l2s: HashMap<ExchangeId, ExchangeChannel<MarketEvent<InstrumentId, OrderBook>>>,
    liquidations: HashMap<ExchangeId, ExchangeChannel<MarketEvent<InstrumentId, Liquidation>>>,
}

impl<InstrumentId> Default for Channels<InstrumentId> {
    fn default() -> Self {
        Self {
            trades: Default::default(),
            l1s: Default::default(),
            l2s: Default::default(),
            liquidations: Default::default(),
        }
    }
}
