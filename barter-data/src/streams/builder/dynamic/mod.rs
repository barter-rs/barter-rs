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
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    Validator,
    channel::{UnboundedRx, UnboundedTx, mpsc_unbounded},
    error::SocketError,
};
use fnv::FnvHashMap;
use futures::{Stream, stream::SelectAll};
use futures_util::{StreamExt, future::try_join_all};
use itertools::Itertools;
use std::{
    fmt::{Debug, Display},
    sync::Arc,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use vecmap::VecMap;

pub mod indexed;

#[derive(Debug)]
pub struct DynamicStreams<InstrumentKey> {
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
        // Validate & dedup Subscription batches
        let batches = validate_batches(subscription_batches)?;

        // Generate required Channels from Subscription batches
        let channels = Channels::try_from(&batches)?;

        let futures =
            batches.into_iter().map(|mut batch| {
                batch.sort_unstable_by_key(|sub| (sub.exchange, sub.kind));
                let by_exchange_by_sub_kind =
                    batch.into_iter().chunk_by(|sub| (sub.exchange, sub.kind));

                let batch_futures =
                    by_exchange_by_sub_kind
                        .into_iter()
                        .map(|((exchange, sub_kind), subs)| {
                            let subs = subs.into_iter().collect::<Vec<_>>();
                            let txs = Arc::clone(&channels.txs);
                            async move {
                                match (exchange, sub_kind) {
                                    (ExchangeId::BinanceSpot, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        BinanceSpot::default(),
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BinanceSpot, SubKind::OrderBooksL1) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        BinanceSpot::default(),
                                                        sub.instrument,
                                                        OrderBooksL1,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.l1s.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BinanceSpot, SubKind::OrderBooksL2) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        BinanceSpot::default(),
                                                        sub.instrument,
                                                        OrderBooksL2,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.l2s.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BinanceFuturesUsd, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        BinanceFuturesUsd::default(),
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BinanceFuturesUsd, SubKind::OrderBooksL1) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::<_, Instrument, _>::new(
                                                        BinanceFuturesUsd::default(),
                                                        sub.instrument,
                                                        OrderBooksL1,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.l1s.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BinanceFuturesUsd, SubKind::OrderBooksL2) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::<_, Instrument, _>::new(
                                                        BinanceFuturesUsd::default(),
                                                        sub.instrument,
                                                        OrderBooksL2,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.l2s.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BinanceFuturesUsd, SubKind::Liquidations) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::<_, Instrument, _>::new(
                                                        BinanceFuturesUsd::default(),
                                                        sub.instrument,
                                                        Liquidations,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.liquidations.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::Bitfinex, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        Bitfinex,
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::Bitmex, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        Bitmex,
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BybitSpot, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        BybitSpot::default(),
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BybitSpot, SubKind::OrderBooksL1) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        BybitSpot::default(),
                                                        sub.instrument,
                                                        OrderBooksL1,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.l1s.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BybitSpot, SubKind::OrderBooksL2) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        BybitSpot::default(),
                                                        sub.instrument,
                                                        OrderBooksL2,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.l2s.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BybitPerpetualsUsd, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        BybitPerpetualsUsd::default(),
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BybitPerpetualsUsd, SubKind::OrderBooksL1) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        BybitSpot::default(),
                                                        sub.instrument,
                                                        OrderBooksL1,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.l1s.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::BybitPerpetualsUsd, SubKind::OrderBooksL2) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        BybitSpot::default(),
                                                        sub.instrument,
                                                        OrderBooksL2,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.l2s.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::Coinbase, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        Coinbase,
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::GateioSpot, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        GateioSpot::default(),
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::GateioFuturesUsd, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        GateioFuturesUsd::default(),
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::GateioFuturesBtc, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        GateioFuturesBtc::default(),
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::GateioPerpetualsUsd, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        GateioPerpetualsUsd::default(),
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::GateioPerpetualsBtc, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        GateioPerpetualsBtc::default(),
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::GateioOptions, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        GateioOptions::default(),
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::Kraken, SubKind::PublicTrades) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        Kraken,
                                                        sub.instrument,
                                                        PublicTrades,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::Kraken, SubKind::OrderBooksL1) => {
                                        init_market_stream(
                                            STREAM_RECONNECTION_POLICY,
                                            subs.into_iter()
                                                .map(|sub| {
                                                    Subscription::new(
                                                        Kraken,
                                                        sub.instrument,
                                                        OrderBooksL1,
                                                    )
                                                })
                                                .collect(),
                                        )
                                        .await
                                        .map(|stream| {
                                            tokio::spawn(stream.forward_to(
                                                txs.l1s.get(&exchange).unwrap().clone(),
                                            ))
                                        })
                                    }
                                    (ExchangeId::Okx, SubKind::PublicTrades) => init_market_stream(
                                        STREAM_RECONNECTION_POLICY,
                                        subs.into_iter()
                                            .map(|sub| {
                                                Subscription::new(Okx, sub.instrument, PublicTrades)
                                            })
                                            .collect(),
                                    )
                                    .await
                                    .map(|stream| {
                                        tokio::spawn(
                                            stream.forward_to(
                                                txs.trades.get(&exchange).unwrap().clone(),
                                            ),
                                        )
                                    }),
                                    (exchange, sub_kind) => {
                                        Err(DataError::Unsupported { exchange, sub_kind })
                                    }
                                }
                            }
                        });

                try_join_all(batch_futures)
            });

        try_join_all(futures).await?;

        Ok(Self {
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

    /// Remove an execution [`PublicTrade`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_trades(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, PublicTrade>>> {
        self.trades.remove(&exchange)
    }

    /// Select and merge every execution [`PublicTrade`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all::select_all).
    pub fn select_all_trades(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, PublicTrade>>> {
        futures_util::stream::select_all::select_all(std::mem::take(&mut self.trades).into_values())
    }

    /// Remove an execution [`OrderBookL1`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_l1s(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, OrderBookL1>>> {
        self.l1s.remove(&exchange)
    }

    /// Select and merge every execution [`OrderBookL1`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all::select_all).
    pub fn select_all_l1s(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, OrderBookL1>>> {
        futures_util::stream::select_all::select_all(std::mem::take(&mut self.l1s).into_values())
    }

    /// Remove an execution [`OrderBookEvent`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_l2s(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, OrderBookEvent>>> {
        self.l2s.remove(&exchange)
    }

    /// Select and merge every execution [`OrderBookEvent`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all::select_all).
    pub fn select_all_l2s(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, OrderBookEvent>>> {
        futures_util::stream::select_all::select_all(std::mem::take(&mut self.l2s).into_values())
    }

    /// Remove an execution [`Liquidation`] `Stream` from the [`DynamicStreams`] collection.
    ///
    /// Note that calling this method will permanently remove this `Stream` from [`Self`].
    pub fn select_liquidations(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, Liquidation>>> {
        self.liquidations.remove(&exchange)
    }

    /// Select and merge every execution [`Liquidation`] `Stream` using
    /// [`SelectAll`](futures_util::stream::select_all::select_all).
    pub fn select_all_liquidations(
        &mut self,
    ) -> SelectAll<UnboundedReceiverStream<MarketStreamResult<InstrumentKey, Liquidation>>> {
        futures_util::stream::select_all::select_all(
            std::mem::take(&mut self.liquidations).into_values(),
        )
    }

    /// Select and merge every execution `Stream` for every data type using [`select_all`](futures_util::stream::select_all::select_all)
    ///
    /// Note that using [`MarketStreamResult<Instrument, DataKind>`] as the `Output` is suitable for most
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
        let Self {
            trades,
            l1s,
            l2s,
            liquidations,
        } = self;

        let trades = trades
            .into_values()
            .map(|stream| stream.map(MarketStreamResult::into).boxed());

        let l1s = l1s
            .into_values()
            .map(|stream| stream.map(MarketStreamResult::into).boxed());

        let l2s = l2s
            .into_values()
            .map(|stream| stream.map(MarketStreamResult::into).boxed());

        let liquidations = liquidations
            .into_values()
            .map(|stream| stream.map(MarketStreamResult::into).boxed());

        let all = trades.chain(l1s).chain(l2s).chain(liquidations);

        futures_util::stream::select_all::select_all(all)
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
        .map(validate_subscriptions::<SubIter, Sub, Instrument>)
        .collect()
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

impl<'a, Instrument> TryFrom<&'a Vec<Vec<Subscription<ExchangeId, Instrument, SubKind>>>>
    for Channels<Instrument::Key>
where
    Instrument: InstrumentData,
{
    type Error = DataError;

    fn try_from(
        value: &'a Vec<Vec<Subscription<ExchangeId, Instrument, SubKind>>>,
    ) -> Result<Self, Self::Error> {
        let mut txs = Txs::default();
        let mut rxs = Rxs::default();

        for sub in value.iter().flatten() {
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
                    if let (None, None) = (txs.l2s.get(&sub.exchange), rxs.l2s.get(&sub.exchange)) {
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
