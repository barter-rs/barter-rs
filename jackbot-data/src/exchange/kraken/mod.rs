use self::{
    channel::KrakenChannel, market::KrakenMarket, message::KrakenMessage,
    spot::l2::KrakenOrderBookL2, subscription::KrakenSubResponse, trade::KrakenTrades,
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

/// OrderBook types for [`Kraken`].
pub mod book;

/// Defines the type that translates a Jackbot [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Jackbot [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`]  specific market used for generating [`Connector::requests`].
pub mod market;

/// [`KrakenMessage`] type for [`Kraken`].
pub mod message;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](jackbot_integration) for [`Kraken`].
pub mod subscription;

/// Public trade types for [`Kraken`].
pub mod trade;

/// Rate limiting utilities for Kraken.
pub mod rate_limit;

/// Futures market modules for Kraken (stub; not yet implemented).
pub mod futures;
/// Spot market modules for Kraken.
pub mod spot;

/// [`Kraken`] server base url.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
pub const BASE_URL_KRAKEN: &str = "wss://ws.kraken.com/";

/// [`Kraken`] execution.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
#[derive(Clone, Default, Debug, DeExchange, SerExchange)]
pub struct Kraken;

impl Connector for Kraken {
    const ID: ExchangeId = ExchangeId::Kraken;
    type Channel = KrakenChannel;
    type Market = KrakenMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = KrakenSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_KRAKEN).map_err(SocketError::UrlParse)
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
                        "event": "subscribe",
                        "pair": [market.as_ref()],
                        "subscription": {
                            "name": channel.as_ref()
                        }
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

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Kraken
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, KrakenTrades>>;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for Kraken
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBooksL2, KrakenOrderBookL2>,
    >;
}

// Add pub use for futures components
pub use futures::{
    KrakenFuturesOrderBookL2, KrakenFuturesOrderBooksL2SnapshotFetcher,
    KrakenFuturesOrderBooksL2Transformer,
};
