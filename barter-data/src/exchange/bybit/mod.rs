use crate::{
    exchange::{
        bybit::{
            channel::BybitChannel, market::BybitMarket, message::BybitMessage,
            subscription::BybitResponse,
        },
        subscription::ExchangeSub,
        Connector, ExchangeId, ExchangeServer, PingInterval, StreamSelector,
    },
    instrument::InstrumentData,
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::{trade::PublicTrades, Map},
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream,
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use serde::de::{Error, Unexpected};
use std::{fmt::Debug, marker::PhantomData, time::Duration};
use tokio::time;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`BybitFuturesUsd`](futures::BybitPerpetualsUsd).
pub mod futures;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// Generic [`BybitPayload<T>`](message::BybitPayload) type common to
/// [`BybitSpot`](spot::BybitSpot)
pub mod message;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`BybitSpot`](spot::BybitSpot).
pub mod spot;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](barter_integration::Validator) common to both [`BybitSpot`](spot::BybitSpot)
/// and [`BybitFuturesUsd`](futures::BybitPerpetualsUsd).
pub mod subscription;

/// Public trade types common to both [`BybitSpot`](spot::BybitSpot) and
/// [`BybitFuturesUsd`](futures::BybitPerpetualsUsd).
pub mod trade;

/// Generic [`Bybit<Server>`](Bybit) exchange.
///
/// ### Notes
/// A `Server` [`ExchangeServer`] implementations exists for
/// [`BybitSpot`](spot::BybitSpot) and [`BybitFuturesUsd`](futures::BybitPerpetualsUsd).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Bybit<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Bybit<Server>
where
    Server: ExchangeServer,
{
    const ID: ExchangeId = Server::ID;
    type Channel = BybitChannel;
    type Market = BybitMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = BybitResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(Server::websocket_url()).map_err(SocketError::UrlParse)
    }

    fn ping_interval() -> Option<PingInterval> {
        Some(PingInterval {
            interval: time::interval(Duration::from_millis(5_000)),
            ping: || {
                WsMessage::Text(
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

        vec![WsMessage::Text(
            serde_json::json!({
                "op": "subscribe",
                "args": stream_names
            })
            .to_string(),
        )]
    }

    fn expected_responses<InstrumentId>(_: &Map<InstrumentId>) -> usize {
        1
    }
}

impl<Instrument, Server> StreamSelector<Instrument, PublicTrades> for Bybit<Server>
where
    Instrument: InstrumentData,
    Server: ExchangeServer + Debug + Send + Sync,
{
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Id, PublicTrades, BybitMessage>>;
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
        let expected = Self::ID.as_str();

        if input == Self::ID.as_str() {
            Ok(Self::default())
        } else {
            Err(Error::invalid_value(Unexpected::Str(input), &expected))
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
        let exchange_id = Self::ID.as_str();
        serializer.serialize_str(exchange_id)
    }
}
