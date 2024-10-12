use std::fmt::Display;
use std::hash::Hash;
use fnv::FnvHashMap;
use futures::Stream;
use futures_util::future::try_join_all;
use futures_util::StreamExt;
use itertools::Itertools;
use barter_integration::error::SocketError;
use barter_integration::model::Exchange;
use barter_integration::model::instrument::kind::InstrumentKind;
use barter_integration::Validator;
use crate::error::DataError;
use crate::exchange::binance::futures::BinanceFuturesUsd;
use crate::exchange::binance::market::BinanceMarket;
use crate::exchange::binance::spot::BinanceSpot;
use crate::exchange::bitfinex::Bitfinex;
use crate::exchange::bitfinex::market::BitfinexMarket;
use crate::exchange::bitmex::Bitmex;
use crate::exchange::bitmex::market::BitmexMarket;
use crate::exchange::bybit::futures::BybitPerpetualsUsd;
use crate::exchange::bybit::market::BybitMarket;
use crate::exchange::bybit::spot::BybitSpot;
use crate::exchange::coinbase::Coinbase;
use crate::exchange::coinbase::market::CoinbaseMarket;
use crate::exchange::{ExchangeId, StreamSelector};
use crate::exchange::binance::channel::BinanceChannel;
use crate::exchange::bitfinex::channel::BitfinexChannel;
use crate::exchange::bitmex::channel::BitmexChannel;
use crate::exchange::bybit::channel::BybitChannel;
use crate::exchange::coinbase::channel::CoinbaseChannel;
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
use crate::streams::consumer::{init_market_stream, MarketStreamEvent, StreamKey, STREAM_RECONNECTION_POLICY};
use crate::subscription::{SubKind, Subscription, SubscriptionKind};
use crate::subscription::book::{OrderBookEvent, OrderBooksL1, OrderBooksL2};
use crate::subscription::liquidation::{Liquidation, Liquidations};
use crate::subscription::trade::{PublicTrade, PublicTrades};

// Todo: dynamic WebSocket manager
pub struct MarketStreamManager<StTrades, StL1s, StL2s, StLiqs> {
    pub trades: FnvHashMap<StreamKey<PublicTrades>, StTrades>,
    pub l1s: FnvHashMap<StreamKey<OrderBooksL1>, StL1s>,
    pub l2s: FnvHashMap<StreamKey<OrderBooksL2>, StL2s>,
    pub liquidations: FnvHashMap<StreamKey<Liquidations>, StLiqs>,
}

impl<StTrades, StL1s, StL2s, StLiqs> MarketStreamManager<StTrades, StL1s, StL2s, StLiqs> {
    // pub async fn init<SubBatchIter, SubIter, Sub, Instrument>(
    //     subscription_batches: SubBatchIter,
    // ) -> Result<Self, DataError>
    // where
    //     SubBatchIter: IntoIterator<Item = SubIter>,
    //     SubIter: IntoIterator<Item = Sub>,
    //     Sub: Into<Subscription<ExchangeId, Instrument, SubKind>>,
    //     Instrument: InstrumentData + Ord + 'static,
    // {
    //     // Validate & dedup Subscription batches
    //     let batches = validate_batches(subscription_batches)?;
    //
    //     let futures = batches
    //         .into_iter()
    //         .map(|mut batch| {
    //             batch.sort_unstable_by_key(|sub| sub.kind);
    //             let by_kind = batch.into_iter().chunk_by(|sub| sub.kind);
    //
    //             by_kind
    //                 .into_iter()
    //                 .map(|(kind, subs)| async move {
    //                     match kind {
    //                         SubKind::PublicTrades => {
    //                             subscribe::<_, _, _, PublicTrades>(subs
    //                                 .into_iter()
    //                                 .map(|Subscription { exchange, instrument, kind }| Subscription {
    //                                     exchange,
    //                                     instrument,
    //                                     kind: PublicTrades,
    //                                 }))
    //                         }
    //                         SubKind::OrderBooksL1 => {
    //                             subscribe::<_, _, _, OrderBooksL1>(subs
    //                                 .into_iter()
    //                                 .map(|Subscription { exchange, instrument, kind }| Subscription {
    //                                     exchange,
    //                                     instrument,
    //                                     kind: OrderBooksL1,
    //                                 }))
    //                         }
    //                         SubKind::OrderBooksL2 => {
    //                             subscribe::<_, _, _, OrderBooksL2>(subs
    //                                 .into_iter()
    //                                 .map(|Subscription { exchange, instrument, kind }| Subscription {
    //                                     exchange,
    //                                     instrument,
    //                                     kind: OrderBooksL2,
    //                                 }))
    //                         }
    //                         SubKind::Liquidations => {
    //                             subscribe::<_, _, _, Liquidations>(subs
    //                                 .into_iter()
    //                                 .map(|Subscription { exchange, instrument, kind }| Subscription {
    //                                     exchange,
    //                                     instrument,
    //                                     kind: Liquidations,
    //                                 }))
    //                         }
    //                         unsupported => {
    //                             panic!("")
    //                         }
    //                     }
    //                 })
    //         })
    //         .flatten();
    //
    //     let x = try_join_all(futures).await?;
    //
    //     for i in x {
    //
    //     }
    //
    //
    // }


    // pub fn subscribe_trades<SubIter, Sub, Instrument>(
    //     &mut self,
    //     subscriptions: SubIter,
    // ) -> StreamKey<PublicTrades>
    // {
    //
    // }

    pub fn select_trades<InstrumentKey>(
        &mut self,
        key: &StreamKey<PublicTrades>
    ) -> Option<StTrades>
    where
        StTrades: Stream<Item = MarketStreamEvent<InstrumentKey, PublicTrade>>
    {
        self.trades.remove(key)
    }

    pub fn select_l1s<InstrumentKey>(
        &mut self,
        key: &StreamKey<OrderBooksL1>
    ) -> Option<StL1s>
    where
        StL1s: Stream<Item = MarketStreamEvent<InstrumentKey, PublicTrade>>
    {
        self.l1s.remove(key)
    }

    pub fn select_l2s<InstrumentKey>(
        &mut self,
        key: &StreamKey<OrderBooksL2>
    ) -> Option<StL2s>
    where
        StL2s: Stream<Item = MarketStreamEvent<InstrumentKey, OrderBookEvent>>
    {
        self.l2s.remove(key)
    }

    pub fn select_liquidations<InstrumentKey>(
        &mut self,
        key: &StreamKey<Liquidations>
    ) -> Option<StLiqs>
    where
        StLiqs: Stream<Item = MarketStreamEvent<InstrumentKey, Liquidation>>
    {
        self.liquidations.remove(key)
    }
}

// pub async fn subscribe_trades<SubIter, Sub, Instrument>(
//     subscriptions: SubIter,
// ) -> Result<FnvHashMap<StreamKey<PublicTrades>, impl Stream>, DataError>
// where
//     SubIter: IntoIterator<Item = Sub>,
//     Sub: Into<Subscription<ExchangeId, Instrument, PublicTrades>>,
//     Instrument: InstrumentData + Ord + 'static,
//     Subscription<BinanceSpot, Instrument, PublicTrades>: Identifier<BinanceChannel> + Identifier<BinanceMarket>,
//     Subscription<BinanceFuturesUsd, Instrument, PublicTrades>: Identifier<BinanceChannel> + Identifier<BinanceMarket>,
//     Subscription<Bitfinex, Instrument, PublicTrades>: Identifier<BitfinexChannel> + Identifier<BitfinexMarket>,
//     Subscription<Bitmex, Instrument, PublicTrades>: Identifier<BitmexChannel> + Identifier<BitmexMarket>,
//     Subscription<BybitSpot, Instrument, PublicTrades>: Identifier<BybitChannel> + Identifier<BybitMarket>,
//     Subscription<BybitPerpetualsUsd, Instrument, PublicTrades>: Identifier<BybitChannel> + Identifier<BybitMarket>,
//     Subscription<Coinbase, Instrument, PublicTrades>: Identifier<CoinbaseChannel> + Identifier<CoinbaseMarket>,
//     Subscription<GateioSpot, Instrument, PublicTrades>: Identifier<GateioChannel> + Identifier<GateioMarket>,
//     Subscription<GateioFuturesUsd, Instrument, PublicTrades>: Identifier<GateioChannel> + Identifier<GateioMarket>,
//     Subscription<GateioFuturesBtc, Instrument, PublicTrades>: Identifier<GateioChannel> + Identifier<GateioMarket>,
//     Subscription<GateioPerpetualsUsd, Instrument, PublicTrades>: Identifier<GateioChannel> + Identifier<GateioMarket>,
//     Subscription<GateioPerpetualsBtc, Instrument, PublicTrades>: Identifier<GateioChannel> + Identifier<GateioMarket>,
//     Subscription<GateioOptions, Instrument, PublicTrades>: Identifier<GateioChannel> + Identifier<GateioMarket>,
//     Subscription<Kraken, Instrument, PublicTrades>: Identifier<KrakenChannel> + Identifier<KrakenMarket>,
//     Subscription<Okx, Instrument, PublicTrades>: Identifier<OkxChannel> + Identifier<OkxMarket>,
// {
//     // Validate & dedup Subscriptions
//     let mut subscriptions = validate_subscriptions::<SubIter, Sub, Instrument, PublicTrades>(subscriptions)?;
//
//     // Group Subscriptions by ExchangeId
//     subscriptions.sort_unstable_by_key(|sub| sub.exchange);
//     let subs_by_exchange = subscriptions
//         .into_iter()
//         .chunk_by(|sub| sub.exchange);
//
//     let futures = subs_by_exchange
//         .into_iter()
//         .map(|(exchange, subs)| {
//             let stream_key = StreamKey {
//                 exchange: Exchange::from(exchange),
//                 kind: PublicTrades,
//             };
//
//             async move {
//
//             }
//         })
//
//
// }

pub async fn subscribe<SubIter, Sub, Instrument, Kind>(
    subscriptions: SubIter,
) -> Result<FnvHashMap<StreamKey<Kind>, impl Stream>, DataError>
where
    SubIter: IntoIterator<Item = Sub>,
    Sub: Into<Subscription<ExchangeId, Instrument, Kind>>,
    Instrument: InstrumentData + Ord + 'static,
    Kind: SubscriptionKind + Ord + Hash + Display + Send + Sync + 'static,
    Kind::Event: Send,
    Subscription<ExchangeId, Instrument, Kind>: Validator + Ord,
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
    let mut subscriptions = validate_subscriptions::<SubIter, Sub, Instrument, Kind>(subscriptions)?;

    // Use first Subscription to determine InstrumentKind & SubKind for StreamKey creation
    let sub_kind = subscriptions
        .first()
        .map(|sub| sub.kind.clone())
        .ok_or(DataError::SubscriptionsEmpty)?;

    // Group Subscriptions by (ExchangeId, SubKind)
    subscriptions.sort_unstable_by_key(|sub| sub.exchange);
    let subs_by_exchange_by_sub_kind = subscriptions
        .into_iter()
        .chunk_by(|sub| sub.exchange);

    let futures = subs_by_exchange_by_sub_kind
        .into_iter()
        .map(|(exchange, subs)| {
            let stream_key = StreamKey {
                exchange: Exchange::from(exchange),
                kind: sub_kind.clone(),
            };

            async move {
                let stream_result = match exchange {
                    ExchangeId::BinanceSpot => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        BinanceSpot::default(),
                                        sub.instrument,
                                        sub.kind
                                    )
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::BinanceFuturesUsd => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| Subscription::<_, Instrument, _>::new(
                                    BinanceFuturesUsd::default(),
                                    sub.instrument,
                                    sub.kind,
                                ))
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::Bitfinex => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(Bitfinex, sub.instrument, sub.kind)
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::Bitmex => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(Bitmex, sub.instrument, sub.kind)
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::BybitSpot => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        BybitSpot::default(),
                                        sub.instrument,
                                        sub.kind,
                                    )
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::BybitPerpetualsUsd => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        BybitPerpetualsUsd::default(),
                                        sub.instrument,
                                        sub.kind,
                                    )
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::Coinbase => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(Coinbase, sub.instrument, sub.kind)
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::GateioSpot => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioSpot::default(),
                                        sub.instrument,
                                        sub.kind,
                                    )
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::GateioFuturesUsd => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioFuturesUsd::default(),
                                        sub.instrument,
                                        sub.kind,
                                    )
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::GateioFuturesBtc => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioFuturesBtc::default(),
                                        sub.instrument,
                                        sub.kind,
                                    )
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::GateioPerpetualsBtc => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioPerpetualsUsd::default(),
                                        sub.instrument,
                                        sub.kind,
                                    )
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::GateioPerpetualsUsd => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioPerpetualsBtc::default(),
                                        sub.instrument,
                                        sub.kind,
                                    )
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::GateioOptions => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(
                                        GateioOptions::default(),
                                        sub.instrument,
                                        sub.kind,
                                    )
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::Kraken => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(Kraken, sub.instrument, sub.kind)
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                    ExchangeId::Okx => {
                        init_market_stream(
                            stream_key.clone(),
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(Okx, sub.instrument, sub.kind)
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                };

                stream_result.map(|stream| (stream_key, stream))
            }
        });

    Ok(try_join_all(futures)
        .await?
        .into_iter()
        .collect()
    )
}

fn validate_batches<SubBatchIter, SubIter, Sub, Instrument>(
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
        .map(validate_subscriptions_sub_kind::<SubIter, Sub, Instrument>)
        .collect()
}

fn validate_subscriptions_sub_kind<SubIter, Sub, Instrument>(
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
