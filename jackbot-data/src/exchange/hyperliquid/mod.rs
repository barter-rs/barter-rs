//! Hyperliquid exchange module.
//!
//! This module will provide market data streaming, normalization, and related features for Hyperliquid.

use self::{
    channel::HyperliquidChannel, market::HyperliquidMarket, subscription::HyperliquidSubResponse,
    trade::HyperliquidTrade,
};
use crate::{
    ExchangeWsStream, NoInitialSnapshots,
    exchange::{Connector, ExchangeSub, PingInterval, StreamSelector},
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

/// L1 order book stream and normalization for Hyperliquid.
// pub mod l1;
/// L2 order book stream and normalization for Hyperliquid.
// pub mod l2;
pub mod book;
/// Channel type for Hyperliquid.
pub mod channel;
/// Futures market modules for Hyperliquid (stub; not yet implemented).
pub mod futures;
/// Liquidations stream and normalization for Hyperliquid.
pub mod liquidation;
/// Market type for Hyperliquid.
pub mod market;
/// Spot market modules for Hyperliquid.
pub mod spot;
/// Subscription response type for Hyperliquid.
pub mod subscription;
/// Public trade types for Hyperliquid.
pub mod trade;

/// Rate limiting utilities for Hyperliquid.
pub mod rate_limit;

/// Hyperliquid WebSocket base URL.
pub const BASE_URL_HYPERLIQUID: &str = "wss://api.hyperliquid.xyz/ws";

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hyperliquid;

impl Connector for Hyperliquid {
    const ID: ExchangeId = ExchangeId::Hyperliquid;
    type Channel = HyperliquidChannel;
    type Market = HyperliquidMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = HyperliquidSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_HYPERLIQUID).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        // Hyperliquid expects a subscription message per market/channel
        exchange_subs
            .into_iter()
            .map(|sub| {
                let (channel, market) = (sub.channel.as_ref(), sub.market.as_ref());
                WsMessage::text(
                    json!({
                        "method": "subscribe",
                        "subscription": { "type": channel, "coin": market }
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}

pub use trade::HyperliquidTrades;

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Hyperliquid
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, PublicTrades, HyperliquidTrades>,
    >;
}

// TODO: Implement public trades, order book L1/L2, liquidations, normalization, etc.

// Placeholder for future Hyperliquid connector/stream types
