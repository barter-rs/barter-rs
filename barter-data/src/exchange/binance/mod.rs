use self::{
    book::l1::BinanceOrderBookL1, channel::BinanceChannel, market::BinanceMarket,
    subscription::BinanceSubResponse, trade::BinanceTrade,
};
use crate::{
    Identifier, IdentifierStatic, NoInitialSnapshots, StreamSelector,
    error::DataError,
    event::MarketEvent,
    exchange::{Connector, ExchangeServer, ExchangeSub},
    init_ws_exchange_stream_with_initial_snapshots,
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::{Map, Subscription, SubscriptionKind, book::OrderBooksL1, trade::PublicTrades},
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{WebSocketSerdeParser, WsMessage},
};
use futures_util::Stream;
use std::{fmt::Debug, future::Future, marker::PhantomData};
use url::Url;

/// OrderBook types common to both [`BinanceSpot`](spot::BinanceSpot) and
/// [`BinanceFuturesUsd`](futures::BinanceFuturesUsd).
pub mod book;

/// Defines the type that translates a Barter [`Subscription`]
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`BinanceFuturesUsd`](futures::BinanceFuturesUsd).
pub mod futures;

/// Defines the type that translates a Barter [`Subscription`]
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`BinanceSpot`](spot::BinanceSpot).
pub mod spot;

/// [`Subscription`] response type and response
/// [`Validator`](barter_integration::Validator) common to both [`BinanceSpot`](spot::BinanceSpot)
/// and [`BinanceFuturesUsd`](futures::BinanceFuturesUsd).
pub mod subscription;

/// Public trade types common to both [`BinanceSpot`](spot::BinanceSpot) and
/// [`BinanceFuturesUsd`](futures::BinanceFuturesUsd).
pub mod trade;

/// Generic [`Binance<Server>`](Binance) exchange.
///
/// ### Notes
/// A `Server` [`ExchangeServer`] implementations exists for
/// [`BinanceSpot`](spot::BinanceSpot) and [`BinanceFuturesUsd`](futures::BinanceFuturesUsd).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Binance<Server> {
    server: PhantomData<Server>,
}

impl<Server> IdentifierStatic<ExchangeId> for Binance<Server>
where
    Server: ExchangeServer,
{
    fn id() -> ExchangeId {
        Server::ID
    }
}

impl<Server> Connector for Binance<Server>
where
    Server: ExchangeServer,
{
    type Channel = BinanceChannel;
    type Market = BinanceMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = BinanceSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(Server::websocket_url()).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        let stream_names = exchange_subs
            .into_iter()
            .map(|sub| {
                // Note:
                // Market must be lowercase when subscribing, but lowercase in general since
                // Binance sends message with uppercase MARKET (eg/ BTCUSDT).
                format!(
                    "{}{}",
                    sub.market.as_ref().to_lowercase(),
                    sub.channel.as_ref()
                )
            })
            .collect::<Vec<String>>();

        vec![WsMessage::text(
            serde_json::json!({
                "method": "SUBSCRIBE",
                "params": stream_names,
                "id": 1
            })
            .to_string(),
        )]
    }

    fn expected_responses<InstrumentKey>(_: &Map<InstrumentKey>) -> usize {
        1
    }
}

impl<Instrument, Server> StreamSelector<Instrument, PublicTrades> for Binance<Server>
where
    Instrument: InstrumentData,
    Server: ExchangeServer + Debug + Send + Sync,
    Subscription<Self, Instrument, PublicTrades>:
        Identifier<BinanceChannel> + Identifier<BinanceMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, PublicTrades>>> + Send,
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
        init_ws_exchange_stream_with_initial_snapshots::<
            Self,
            Instrument,
            PublicTrades,
            WebSocketSerdeParser,
            StatelessTransformer<Self, Instrument::Key, PublicTrades, BinanceTrade>,
            NoInitialSnapshots,
        >(subscriptions)
    }
}

impl<Instrument, Server> StreamSelector<Instrument, OrderBooksL1> for Binance<Server>
where
    Instrument: InstrumentData,
    Server: ExchangeServer + Debug + Send + Sync,
    Subscription<Self, Instrument, OrderBooksL1>:
        Identifier<BinanceChannel> + Identifier<BinanceMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, OrderBooksL1>>>,
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
        init_ws_exchange_stream_with_initial_snapshots::<
            Self,
            Instrument,
            OrderBooksL1,
            WebSocketSerdeParser,
            StatelessTransformer<Self, Instrument::Key, OrderBooksL1, BinanceOrderBookL1>,
            NoInitialSnapshots,
        >(subscriptions)
    }
}

impl<'de, Server> serde::Deserialize<'de> for Binance<Server>
where
    Server: ExchangeServer,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let input = <String as serde::Deserialize>::deserialize(deserializer)?;

        if input.as_str() == Self::id().as_str() {
            Ok(Self::default())
        } else {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(input.as_str()),
                &Self::id().as_str(),
            ))
        }
    }
}

impl<Server> serde::Serialize for Binance<Server>
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
