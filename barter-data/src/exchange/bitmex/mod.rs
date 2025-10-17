use crate::{
    ExchangeWsStream, NoInitialSnapshots,
    exchange::{
        Connector, StreamSelector,
        bitmex::{
            channel::BitmexChannel, market::BitmexMarket, subscription::BitmexSubResponse,
            trade::BitmexTrade,
        },
        subscription::ExchangeSub,
    },
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::{Map, trade::PublicTrades},
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{WebSocketSerdeParser, WsMessage},
};
use derive_more::Display;
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

/// Public trade types for [`Bitmex`].
pub mod trade;

/// Convenient type alias for a Bitmex [`ExchangeWsStream`] using [`WebSocketSerdeParser`](barter_integration::protocol::websocket::WebSocketSerdeParser).
pub type BitmexWsStream<Transformer> = ExchangeWsStream<WebSocketSerdeParser, Transformer>;

/// [`Bitmex`] server base url.
///
/// See docs: <https://www.bitmex.com/app/wsAPI>
pub const BASE_URL_BITMEX: &str = "wss://ws.bitmex.com/realtime";

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Display)]
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

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Bitmex
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
        BitmexWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, BitmexTrade>>;
}

impl<'de> serde::Deserialize<'de> for Bitmex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let input = <&str as serde::Deserialize>::deserialize(deserializer)?;
        if input == Self::ID.as_str() {
            Ok(Self)
        } else {
            Err(Error::invalid_value(
                Unexpected::Str(input),
                &Self::ID.as_str(),
            ))
        }
    }
}

impl serde::Serialize for Bitmex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(Self::ID.as_str())
    }
}
