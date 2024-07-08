use crate::{
    exchange::{
        bitmex::{
            channel::BitmexChannel, market::BitmexMarket, subscription::BitmexSubResponse,
            trade::BitmexTrade,
        },
        subscription::ExchangeSub,
        Connector, ExchangeId, StreamSelector,
    },
    instrument::InstrumentData,
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::{trade::PublicTrades, Map},
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream,
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use serde::de::{Error, Unexpected};
use std::fmt::Debug;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// Generic [`BitmexMessage<T>`](message::BitmexMessage)
pub mod message;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](barter_integration::Validator) for [`Bitmex`].
pub mod subscription;

/// Public trade types for [`Bitmex`](Bitmex)
pub mod trade;

/// [`Bitmex`] server base url.
///
/// See docs: <https://www.bitmex.com/app/wsAPI>
pub const BASE_URL_BITMEX: &str = "wss://ws.bitmex.com/realtime";

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Bitmex;

impl Connector for Bitmex {
    const ID: ExchangeId = ExchangeId::Bitmex;
    type Channel = BitmexChannel;
    type Market = BitmexMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = BitmexSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_BITMEX).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        let stream_names = exchange_subs
            .into_iter()
            .map(|sub| format!("{}:{}", sub.channel.as_ref(), sub.market.as_ref(),))
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

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Bitmex
where
    Instrument: InstrumentData,
{
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Id, PublicTrades, BitmexTrade>>;
}

impl<'de> serde::Deserialize<'de> for Bitmex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let input = <&str as serde::Deserialize>::deserialize(deserializer)?;
        let expected = Self::ID.as_str();

        if input == Self::ID.as_str() {
            Ok(Self)
        } else {
            Err(Error::invalid_value(Unexpected::Str(input), &expected))
        }
    }
}

impl serde::Serialize for Bitmex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let exchange_id = Self::ID.as_str();
        serializer.serialize_str(exchange_id)
    }
}
