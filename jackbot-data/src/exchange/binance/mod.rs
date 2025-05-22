//! Exchange module for Binance. Implements all required traits and re-exports submodules.
use self::{
    channel::BinanceChannel, market::BinanceMarket, subscription::BinanceSubResponse,
    trade::BinanceTrade,
};
use crate::{
    ExchangeWsStream, NoInitialSnapshots,
    exchange::{Connector, ExchangeServer, ExchangeSub, StreamSelector},
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::{Map, trade::PublicTrades},
    transformer::stateless::StatelessTransformer,
};
use jackbot_instrument::exchange::ExchangeId;
use jackbot_integration::{error::SocketError, protocol::websocket::WsMessage};
use std::{fmt::Debug, marker::PhantomData, time::Duration};
use url::Url;

/// OrderBook types common to both [`BinanceSpot`](spot::BinanceSpot) and
/// [`BinanceFuturesUsd`](futures::BinanceFuturesUsd).
pub mod book;

/// Defines the type that translates a Jackbot [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`BinanceFuturesUsd`](futures::BinanceFuturesUsd).
pub mod futures;

/// Defines the type that translates a Jackbot [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// [`ExchangeServer`] and [`StreamSelector`] implementations for
/// [`BinanceSpot`](spot::BinanceSpot).
pub mod spot;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](jackbot_integration::Validator) common to both [`BinanceSpot`](spot::BinanceSpot)
/// and [`BinanceFuturesUsd`](futures::BinanceFuturesUsd).
pub mod subscription;

/// Public trade types common to both [`BinanceSpot`](spot::BinanceSpot) and
/// [`BinanceFuturesUsd`](futures::BinanceFuturesUsd).
pub mod trade;

/// Generic [`Binance<Server>`](Binance) execution.
///
/// ### Notes
/// A `Server` [`ExchangeServer`] implementations exists for
/// [`BinanceSpot`](spot::BinanceSpot) and [`BinanceFuturesUsd`](futures::BinanceFuturesUsd).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Binance<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Binance<Server>
where
    Server: ExchangeServer,
{
    const ID: ExchangeId = Server::ID;
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

    fn ping_interval() -> Option<PingInterval> {
        Some(PingInterval {
            interval: tokio::time::interval(Duration::from_secs(30)),
            ping: || WsMessage::Ping(Vec::new().into()),
        })
    }

    fn heartbeat_interval() -> Option<Duration> {
        Some(Duration::from_secs(90))
    }

    fn expected_responses<InstrumentKey>(_: &Map<InstrumentKey>) -> usize {
        1
    }
}

impl<Instrument, Server> StreamSelector<Instrument, PublicTrades> for Binance<Server>
where
    Instrument: InstrumentData,
    Server: ExchangeServer + Debug + Send + Sync,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, BinanceTrade>>;
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

impl<Server> serde::Serialize for Binance<Server>
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
