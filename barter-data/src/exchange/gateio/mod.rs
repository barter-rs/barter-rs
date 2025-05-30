use self::{channel::GateioChannel, market::GateioMarket, subscription::GateioSubResponse};
use crate::{
    ExchangeWsStream, NoInitialSnapshots,
    exchange::{Connector, ExchangeServer, subscription::ExchangeSub},
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::book::{OrderBooksL1, OrderBooksL2},
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use book::{
    l1::GateioOrderBookL1,
    l2::{GateioSpotOrderBooksL2SnapshotFetcher, GateioSpotOrderBooksL2Transformer},
};
use serde_json::json;
use spot::GateioServerSpot;
use std::{fmt::Debug, marker::PhantomData};
use url::Url;

use super::StreamSelector;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// [`ExchangeServer`] and [`StreamSelector`](super::StreamSelector) implementations for
/// [`GateioSpot`](spot::GateioSpot).
pub mod spot;

/// [`ExchangeServer`] and [`StreamSelector`](super::StreamSelector) implementations for
/// [`GateioFutureUsd`](future::GateioFuturesUsd) and
/// [`GateioFutureBtc`](future::GateioFuturesBtc).
pub mod future;

/// [`ExchangeServer`] and [`StreamSelector`](super::StreamSelector) implementations for
/// [`GateioPerpetualUsdt`](perpetual::GateioPerpetualsUsd) and
/// [`GateioPerpetualBtc`](perpetual::GateioPerpetualsBtc).
pub mod perpetual;

/// [`ExchangeServer`] and [`StreamSelector`](super::StreamSelector) implementations for
/// [`GateioOptions`](option::GateioOptions)
pub mod option;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// Generic [`GateioMessage<T>`](message::GateioMessage) type common to
/// [`GateioSpot`](spot::GateioSpot), [`GateioPerpetualUsdt`](perpetual::GateioPerpetualsUsd)
/// and [`GateioPerpetualBtc`](perpetual::GateioPerpetualsBtc).
pub mod message;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](barter_integration) common to [`GateioSpot`](spot::GateioSpot),
/// [`GateioPerpetualUsdt`](perpetual::GateioPerpetualsUsd) and
/// [`GateioPerpetualBtc`](perpetual::GateioPerpetualsBtc).
pub mod subscription;

pub mod book;
/// Generic [`Gateio<Server>`](Gateio) execution.
///
/// ### Notes
/// A `Server` [`ExchangeServer`] implementations exists for
/// [`GateioSpot`](spot::GateioSpot), [`GateioPerpetualUsdt`](perpetual::GateioPerpetualsUsd) and
/// [`GateioPerpetualBtc`](perpetual::GateioPerpetualsBtc).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Gateio<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Gateio<Server>
where
    Server: ExchangeServer,
{
    const ID: ExchangeId = Server::ID;
    type Channel = GateioChannel;
    type Market = GateioMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = GateioSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(Server::websocket_url()).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::text(
                    json!({
                        "time": chrono::Utc::now().timestamp_millis(),
                        "channel": channel.as_ref(),
                        "event": "subscribe",
                        "payload": market.as_str_vec()
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}

impl<'de, Server> serde::Deserialize<'de> for Gateio<Server>
where
    Server: ExchangeServer,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let input = <String as serde::Deserialize>::deserialize(deserializer)?;
        if input.as_str() == Self::ID.as_str() {
            Ok(Self::default())
        } else {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(input.as_str()),
                &Self::ID.as_str(),
            ))
        }
    }
}

impl<Server> serde::Serialize for Gateio<Server>
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

impl<Instrument, Server> StreamSelector<Instrument, OrderBooksL1> for Gateio<Server>
where
    Instrument: InstrumentData,
    Server: ExchangeServer + Debug + Send + Sync,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBooksL1, GateioOrderBookL1>,
    >;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for Gateio<GateioServerSpot>
where
    Instrument: InstrumentData,
{
    type SnapFetcher = GateioSpotOrderBooksL2SnapshotFetcher;
    type Stream = ExchangeWsStream<GateioSpotOrderBooksL2Transformer<Instrument::Key>>;
}
