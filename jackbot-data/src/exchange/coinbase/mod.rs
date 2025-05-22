use self::{
    channel::CoinbaseChannel, market::CoinbaseMarket, spot::l2::CoinbaseOrderBookL2Update,
    subscription::CoinbaseSubResponse, trade::CoinbaseTrade,
};
use crate::{
    ExchangeWsStream, NoInitialSnapshots,
    exchange::{
        Connector, ExchangeSub, StreamSelector, PingInterval,
        DEFAULT_PING_INTERVAL, DEFAULT_HEARTBEAT_INTERVAL,
    },
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::{book::OrderBooksL2, trade::PublicTrades},
    transformer::stateless::StatelessTransformer,
};
use derive_more::Display;
use jackbot_instrument::exchange::ExchangeId;
use jackbot_integration::{error::SocketError, protocol::websocket::WsMessage};
use jackbot_macro::{DeExchange, SerExchange};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use url::Url;

/// OrderBook types for [`Coinbase`].
pub mod book;

/// Defines the type that translates a Jackbot [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Jackbot [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](jackbot_integration::Validator) for [`Coinbase`].
pub mod subscription;

/// Public trade types for [`Coinbase`].
pub mod trade;

/// [`Coinbase`] server base url.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
pub const BASE_URL_COINBASE: &str = "wss://ws-feed.execution.coinbase.com";

/// [`Coinbase`] execution.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
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
    DeExchange,
    SerExchange,
)]
pub struct Coinbase;

impl Connector for Coinbase {
    const ID: ExchangeId = ExchangeId::Coinbase;
    type Channel = CoinbaseChannel;
    type Market = CoinbaseMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = CoinbaseSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_COINBASE).map_err(SocketError::UrlParse)
    }

    fn ping_interval() -> Option<PingInterval> {
        Some(PingInterval {
            interval: tokio::time::interval(DEFAULT_PING_INTERVAL),
            ping: || WsMessage::text("ping"),
        })
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::text(
                    json!({
                        "type": "subscribe",
                        "product_ids": [market.as_ref()],
                        "channels": [channel.as_ref()],
                    })
                    .to_string(),
                )
            })
            .collect()
    }

    fn heartbeat_interval() -> Option<Duration> {
        Some(DEFAULT_HEARTBEAT_INTERVAL)
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Coinbase
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, CoinbaseTrade>>;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for Coinbase
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBooksL2, CoinbaseOrderBookL2Update>,
    >;
}

pub mod spot;
