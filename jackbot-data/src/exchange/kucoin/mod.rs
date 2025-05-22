//! Exchange module for Kucoin. Implements all required traits and re-exports submodules.
//!
//! This module provides market data streaming, normalization, and related features for Kucoin.

use self::{
    channel::KucoinChannel, market::KucoinMarket, subscription::KucoinSubscription,
    trade::KucoinTrade,
};
use crate::{
    ExchangeWsStream, NoInitialSnapshots,
    exchange::{
        Connector, ExchangeSub, StreamSelector, PingInterval,
        DEFAULT_PING_INTERVAL, DEFAULT_HEARTBEAT_INTERVAL,
    },
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
};
use jackbot_instrument::exchange::ExchangeId;
use jackbot_integration::{error::SocketError, protocol::websocket::WsMessage};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use url::Url;

pub mod book;
pub mod channel;
/// Futures market modules for Kucoin.
pub mod futures;
pub mod liquidation;
pub mod market;
/// Spot market modules for Kucoin.
pub mod spot;
pub mod subscription;
pub mod trade;
pub mod rate_limit;

/// Kucoin WebSocket base URL.
pub const BASE_URL_KUCOIN: &str = "wss://ws-api.kucoin.com/endpoint";

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kucoin;

impl Connector for Kucoin {
    const ID: ExchangeId = ExchangeId::Kucoin;
    type Channel = KucoinChannel;
    type Market = KucoinMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = KucoinSubscription;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_KUCOIN).map_err(SocketError::UrlParse)
    }

    fn ping_interval() -> Option<PingInterval> {
        Some(PingInterval {
            interval: tokio::time::interval(DEFAULT_PING_INTERVAL),
            ping: || WsMessage::text("ping"),
        })
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        // Kucoin expects a subscription message per market/channel
        exchange_subs
            .into_iter()
            .map(|sub| {
                let (channel, market) = (sub.channel.as_str(), sub.market.normalize());
                WsMessage::text(
                    json!({
                        "id": "Jackbot-kucoin-subscribe",
                        "type": "subscribe",
                        "topic": channel,
                        "privateChannel": false,
                        "response": true,
                        "market": market
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

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Kucoin
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, KucoinTrade>>;
}
