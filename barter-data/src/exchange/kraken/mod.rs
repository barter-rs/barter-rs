use self::{
    channel::KrakenChannel, market::KrakenMarket, subscription::KrakenSubResponse,
};
use crate::{
    ExchangeWsStream,
    exchange::{Connector, ExchangeServer, ExchangeSub},
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{WebSocketSerdeParser, WsMessage},
};
use derive_more::Display;
use serde_json::json;
use std::marker::PhantomData;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`KrakenFuturesUsd`](futures::KrakenFuturesUsd).
pub mod futures;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`]  specific market used for generating [`Connector::requests`].
pub mod market;

/// [`KrakenMessage`] type for [`Kraken`].
pub mod message;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`KrakenSpot`](spot::KrakenSpot).
pub mod spot;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](barter_integration) for [`Kraken`].
pub mod subscription;

/// Convenient type alias for a Kraken [`ExchangeWsStream`] using [`WebSocketSerdeParser`](barter_integration::protocol::websocket::WebSocketSerdeParser).
pub type KrakenWsStream<Transformer> = ExchangeWsStream<WebSocketSerdeParser, Transformer>;

/// [`Kraken`] exchange.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Default,
    Display,
)]
pub struct KrakenExchange<Server> {
    server: PhantomData<Server>,
}

pub type Kraken = KrakenExchange<spot::KrakenServerSpot>;
pub type KrakenSpot = Kraken;
pub type KrakenFuturesUsd = KrakenExchange<futures::KrakenServerFuturesUsd>;



impl<'de, Server> serde::Deserialize<'de> for KrakenExchange<Server>
where
    Server: ExchangeServer,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let input = <String as serde::Deserialize>::deserialize(deserializer)?;
        let expected = Self::ID.as_str();

        if input.as_str() == expected {
            Ok(Self::default())
        } else {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(input.as_str()),
                &expected,
            ))
        }
    }
}

impl<Server> serde::Serialize for KrakenExchange<Server>
where
    Server: ExchangeServer,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(Self::ID.as_str())
    }
}
