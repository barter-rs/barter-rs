use std::fmt::Display;
use futures::Stream;
use futures_util::future::try_join_all;
use futures_util::StreamExt;
use itertools::Itertools;
use tokio_stream::StreamMap;
use vecmap::VecMap;
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
use crate::streams::consumer::{init_market_stream, STREAM_RECONNECTION_POLICY};
use crate::subscription::{SubKind, Subscription, SubscriptionKind};

// Todo: dynamic WebSocket manager
pub struct MarketDataManager<StTrades> {
    pub subscriber: MarketDataSubscriber,


    pub trades: StreamMap<MarketStreamKey, StTrades>,
}

// pub struct Streams {
//     pub trades: VecMap<ExchangeId, StTrades>,
//     pub l1s: VecMap<ExchangeId, StTrades>,
//     pub l2s: VecMap<ExchangeId, StTrades>,
//     pub liquidations: VecMap<ExchangeId, StTrades>,
// }


pub struct MarketDataSubscriber;

impl MarketDataSubscriber {

    pub async fn subscribe_trades<SubIter, Sub, Instrument, Kind>(
        &mut self,
        subscriptions: SubIter,
    ) -> Result<VecMap<ExchangeId, impl Stream>, DataError>
    where
        SubIter: IntoIterator<Item = Sub>,
        Sub: Into<Subscription<ExchangeId, Instrument, Kind>>,
        Instrument: InstrumentData + Ord + 'static,
        Kind: SubscriptionKind + Ord + Display + Send + 'static,
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

        // Group Subscriptions by (ExchangeId, SubKind)
        subscriptions.sort_unstable_by_key(|sub| sub.exchange);
        let subs_by_exchange_by_sub_kind = subscriptions
            .into_iter()
            .chunk_by(|sub| sub.exchange);

        let futures = subs_by_exchange_by_sub_kind
            .into_iter()
            .map(|(exchange, subs)| async move {
                let stream_result = match exchange {
                    ExchangeId::BinanceSpot => {
                        init_market_stream(
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
                            STREAM_RECONNECTION_POLICY,
                            subs.into_iter()
                                .map(|sub| {
                                    Subscription::new(Okx, sub.instrument, sub.kind)
                                })
                                .collect(),
                        ).await.map(StreamExt::boxed)
                    }
                };

                stream_result.map(|stream| (exchange, stream))
            });

        Ok(try_join_all(futures)
            .await?
            .into_iter()
            .collect()
        )
    }


    // pub async fn subscribe<SubIter, Sub, Instrument>(
    //     &mut self,
    //     subscriptions: SubIter
    // ) -> Result<Vec<impl Stream>, DataError>
    // where
    //     SubIter: IntoIterator<Item = Sub>,
    //     Sub: Into<Subscription<ExchangeId, Instrument, SubKind>>,
    //     Instrument: InstrumentData + Ord + 'static,
    //     Subscription<BinanceSpot, Instrument, PublicTrades>: Identifier<BinanceMarket>,
    //     Subscription<BinanceSpot, Instrument, PublicTrades>: Identifier<BinanceMarket>,
    //     Subscription<BinanceSpot, Instrument, OrderBooksL1>: Identifier<BinanceMarket>,
    //     Subscription<BinanceFuturesUsd, Instrument, PublicTrades>: Identifier<BinanceMarket>,
    //     Subscription<BinanceFuturesUsd, Instrument, OrderBooksL1>: Identifier<BinanceMarket>,
    //     Subscription<BinanceFuturesUsd, Instrument, Liquidations>: Identifier<BinanceMarket>,
    //     Subscription<Bitfinex, Instrument, PublicTrades>: Identifier<BitfinexMarket>,
    //     Subscription<Bitmex, Instrument, PublicTrades>: Identifier<BitmexMarket>,
    //     Subscription<BybitSpot, Instrument, PublicTrades>: Identifier<BybitMarket>,
    //     Subscription<BybitPerpetualsUsd, Instrument, PublicTrades>: Identifier<BybitMarket>,
    //     Subscription<Coinbase, Instrument, PublicTrades>: Identifier<CoinbaseMarket>,
    //     Subscription<GateioSpot, Instrument, PublicTrades>: Identifier<GateioMarket>,
    //     Subscription<GateioFuturesUsd, Instrument, PublicTrades>: Identifier<GateioMarket>,
    //     Subscription<GateioFuturesBtc, Instrument, PublicTrades>: Identifier<GateioMarket>,
    //     Subscription<GateioPerpetualsUsd, Instrument, PublicTrades>: Identifier<GateioMarket>,
    //     Subscription<GateioPerpetualsBtc, Instrument, PublicTrades>: Identifier<GateioMarket>,
    //     Subscription<GateioOptions, Instrument, PublicTrades>: Identifier<GateioMarket>,
    //     Subscription<Kraken, Instrument, PublicTrades>: Identifier<KrakenMarket>,
    //     Subscription<Kraken, Instrument, OrderBooksL1>: Identifier<KrakenMarket>,
    //     Subscription<Okx, Instrument, PublicTrades>: Identifier<OkxMarket>,
    // {
    //     // Validate & dedup Subscriptions
    //     let mut subscriptions = validate_subscriptions::<SubIter, Sub, Instrument>(subscriptions)?;
    //
    //     // Group Subscriptions by (ExchangeId, SubKind)
    //     subscriptions.sort_unstable_by_key(|sub| (sub.exchange, sub.kind));
    //     let subs_by_exchange_by_sub_kind = subscriptions
    //         .into_iter()
    //         .chunk_by(|sub| (sub.exchange, sub.kind));
    //
    //     let futures = subs_by_exchange_by_sub_kind
    //         .into_iter()
    //         .map(|((exchange, sub_kind), subs)| async move {
    //             match (exchange, sub_kind) {
    //                 (ExchangeId::BinanceSpot, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(
    //                                     BinanceSpot::default(),
    //                                     sub.instrument,
    //                                     PublicTrades
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::BinanceSpot, SubKind::OrderBooksL1) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(
    //                                     BinanceSpot::default(),
    //                                     sub.instrument,
    //                                     OrderBooksL1
    //                                 )
    //                             })
    //                             .collect()
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::BinanceFuturesUsd, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| Subscription::new(
    //                                 BinanceFuturesUsd::default(),
    //                                 sub.instrument,
    //                                 PublicTrades,
    //                             ))
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::BinanceFuturesUsd, SubKind::OrderBooksL1) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::<_, Instrument, _>::new(
    //                                     BinanceFuturesUsd::default(),
    //                                     sub.instrument,
    //                                     OrderBooksL1,
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::BinanceFuturesUsd, SubKind::Liquidations) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::<_, Instrument, _>::new(
    //                                     BinanceFuturesUsd::default(),
    //                                     sub.instrument,
    //                                     Liquidations,
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::Bitfinex, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(Bitfinex, sub.instrument, PublicTrades)
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::Bitmex, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(Bitmex, sub.instrument, PublicTrades)
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::BybitSpot, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(
    //                                     BybitSpot::default(),
    //                                     sub.instrument,
    //                                     PublicTrades,
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::BybitPerpetualsUsd, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(
    //                                     BybitPerpetualsUsd::default(),
    //                                     sub.instrument,
    //                                     PublicTrades,
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::Coinbase, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(Coinbase, sub.instrument, PublicTrades)
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::GateioSpot, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(
    //                                     GateioSpot::default(),
    //                                     sub.instrument,
    //                                     PublicTrades,
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::GateioFuturesUsd, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(
    //                                     GateioFuturesUsd::default(),
    //                                     sub.instrument,
    //                                     PublicTrades,
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::GateioFuturesBtc, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(
    //                                     GateioFuturesBtc::default(),
    //                                     sub.instrument,
    //                                     PublicTrades,
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::GateioPerpetualsUsd, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(
    //                                     GateioPerpetualsUsd::default(),
    //                                     sub.instrument,
    //                                     PublicTrades,
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::GateioPerpetualsBtc, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(
    //                                     GateioPerpetualsBtc::default(),
    //                                     sub.instrument,
    //                                     PublicTrades,
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::GateioOptions, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(
    //                                     GateioOptions::default(),
    //                                     sub.instrument,
    //                                     PublicTrades,
    //                                 )
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::Kraken, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(Kraken, sub.instrument, PublicTrades)
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::Kraken, SubKind::OrderBooksL1) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(Kraken, sub.instrument, OrderBooksL1)
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (ExchangeId::Okx, SubKind::PublicTrades) => {
    //                     init_market_stream(
    //                         STREAM_RECONNECTION_POLICY,
    //                         subs.into_iter()
    //                             .map(|sub| {
    //                                 Subscription::new(Okx, sub.instrument, PublicTrades)
    //                             })
    //                             .collect(),
    //                     ).await.map(StreamExt::boxed)
    //                 }
    //                 (exchange, sub_kind) => {
    //                     return Err(DataError::Unsupported { exchange, sub_kind })
    //                 }
    //             }
    //         });
    //
    //     let result = try_join_all(futures).await;
    //     result
    // }
}

pub struct Connections {
    pub streams: VecMap<MarketStreamKey, ConnectionState>
}

pub struct ConnectionState {

}

pub struct MarketStreamKey {
    pub id: u8,
    pub exchange: Exchange,
    pub kind_instrument: InstrumentKind,
    pub kind_subscription: SubKind,
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
