use crate::{
    Identifier, IdentifierStatic, NoInitialSnapshots, StreamSelector,
    error::DataError,
    event::MarketEvent,
    exchange::{
        Connector, ExchangeServer, PingInterval,
        bybit::{channel::BybitChannel, market::BybitMarket, subscription::BybitResponse},
        subscription::ExchangeSub,
    },
    init_ws_exchange_stream,
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::{
        Map, Subscription, SubscriptionKind,
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{protocol::websocket::WsMessage, serde::de::DeJson};
use book::{BybitOrderBookMessage, l2::BybitOrderBooksL2Transformer};
use futures_util::Stream;
use serde::de::{Error, Unexpected};
use std::{fmt::Debug, future::Future, marker::PhantomData, time::Duration};
use tokio::time;
use trade::BybitTrade;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`]
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`BybitFuturesUsd`](futures::BybitPerpetualsUsd).
pub mod futures;

/// Defines the type that translates a Barter [`Subscription`]
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// Generic [`BybitPayload<T>`](message::BybitPayload) type common to
/// [`BybitSpot`](spot::BybitSpot)
pub mod message;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`BybitSpot`](spot::BybitSpot).
pub mod spot;

/// [`Subscription`] response type and response
/// [`Validator`](barter_integration::Validator) common to both [`BybitSpot`](spot::BybitSpot)
/// and [`BybitFuturesUsd`](futures::BybitPerpetualsUsd).
pub mod subscription;

/// Public trade types common to both [`BybitSpot`](spot::BybitSpot) and
/// [`BybitFuturesUsd`](futures::BybitPerpetualsUsd).
pub mod trade;

/// Orderbook types common to both [`BybitSpot`](spot::BybitSpot) and
/// [`BybitFuturesUsd`](futures::BybitPerpetualsUsd).
pub mod book;

/// Generic [`Bybit<Server>`](Bybit) exchange.
///
/// ### Notes
/// A `Server` [`ExchangeServer`] implementations exists for
/// [`BybitSpot`](spot::BybitSpot) and [`BybitFuturesUsd`](futures::BybitPerpetualsUsd).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Bybit<Server> {
    server: PhantomData<Server>,
}

impl<Server> IdentifierStatic<ExchangeId> for Bybit<Server>
where
    Server: ExchangeServer,
{
    fn id() -> ExchangeId {
        Server::ID
    }
}

impl<Server> Connector for Bybit<Server>
where
    Server: ExchangeServer,
{
    type Channel = BybitChannel;
    type Market = BybitMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = BybitResponse;

    fn url() -> Result<Url, url::ParseError> {
        Url::parse(Server::websocket_url())
    }

    fn ping_interval() -> Option<PingInterval> {
        Some(PingInterval {
            interval: time::interval(Duration::from_millis(5_000)),
            ping: || {
                WsMessage::text(
                    serde_json::json!({
                        "op": "ping",
                    })
                    .to_string(),
                )
            },
        })
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        let stream_names = exchange_subs
            .into_iter()
            .map(|sub| format!("{}.{}", sub.channel.as_ref(), sub.market.as_ref(),))
            .collect::<Vec<String>>();

        vec![WsMessage::text(
            serde_json::json!({
                "op": "subscribe",
                "args": stream_names
            })
            .to_string(),
        )]
    }

    fn expected_responses<InstrumentKey>(_: &Map<InstrumentKey>) -> usize {
        1
    }
}

impl<Instrument, Server> StreamSelector<Instrument, PublicTrades> for Bybit<Server>
where
    Instrument: InstrumentData,
    Server: ExchangeServer + Debug + Send + Sync,
    Subscription<Self, Instrument, PublicTrades>:
        Identifier<BybitChannel> + Identifier<BybitMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, PublicTrades>>> + Send,
        stream_timeout: std::time::Duration,
    ) -> impl Future<
        Output = Result<
            impl Stream<
                Item = Result<
                    MarketEvent<Instrument::Key, <PublicTrades as SubscriptionKind>::Event>,
                    DataError,
                >,
            >,
            DataError,
        >,
    > {
        init_ws_exchange_stream::<
            Self,
            Instrument,
            PublicTrades,
            DeJson,
            StatelessTransformer<Self, Instrument::Key, PublicTrades, BybitTrade>,
            NoInitialSnapshots,
        >(subscriptions, stream_timeout)
    }
}

impl<Instrument, Server> StreamSelector<Instrument, OrderBooksL1> for Bybit<Server>
where
    Instrument: InstrumentData,
    Server: ExchangeServer + Debug + Send + Sync,
    Subscription<Self, Instrument, OrderBooksL1>:
        Identifier<BybitChannel> + Identifier<BybitMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, OrderBooksL1>>>,
        stream_timeout: std::time::Duration,
    ) -> impl Future<
        Output = Result<
            impl Stream<
                Item = Result<
                    MarketEvent<Instrument::Key, <OrderBooksL1 as SubscriptionKind>::Event>,
                    DataError,
                >,
            >,
            DataError,
        >,
    > {
        init_ws_exchange_stream::<
            Self,
            Instrument,
            OrderBooksL1,
            DeJson,
            StatelessTransformer<Self, Instrument::Key, OrderBooksL1, BybitOrderBookMessage>,
            NoInitialSnapshots,
        >(subscriptions, stream_timeout)
    }
}

impl<Instrument, Server> StreamSelector<Instrument, OrderBooksL2> for Bybit<Server>
where
    Instrument: InstrumentData,
    Server: ExchangeServer + Debug + Send + Sync,
    Subscription<Self, Instrument, OrderBooksL2>:
        Identifier<BybitChannel> + Identifier<BybitMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, OrderBooksL2>>> + Send,
        stream_timeout: std::time::Duration,
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
        init_ws_exchange_stream::<
            Self,
            Instrument,
            OrderBooksL2,
            DeJson,
            BybitOrderBooksL2Transformer<Instrument::Key>,
            NoInitialSnapshots,
        >(subscriptions, stream_timeout)
    }
}

impl<'de, Server> serde::Deserialize<'de> for Bybit<Server>
where
    Server: ExchangeServer,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let input = <&str as serde::Deserialize>::deserialize(deserializer)?;

        if input == Self::id().as_str() {
            Ok(Self::default())
        } else {
            Err(Error::invalid_value(
                Unexpected::Str(input),
                &Self::id().as_str(),
            ))
        }
    }
}

impl<Server> serde::Serialize for Bybit<Server>
where
    Server: ExchangeServer,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(Self::id().as_str())
    }
}
