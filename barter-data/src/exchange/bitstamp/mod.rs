use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{WebSocketSerdeParser, WsMessage},
};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    ExchangeWsStream,
    exchange::{
        Connector,
        bitstamp::{
            channel::BitstampChannel, market::BitstampMarket, subscription::BitstampSubResponse,
        },
        subscription::ExchangeSub,
    },
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::Map,
};

pub mod book;
pub mod channel;
pub mod market;
pub mod message;
pub mod subscription;

/// Websocket url
pub const WEBSOCKET_BASE_URL_BITSTAMP: &str = "wss://ws.bitstamp.net/";

/// Convenient type alias for a Bitstamp [`ExchangeWsStream`] using [`WebSocketSerdeParser`].
pub type BitstampWsStream<Transformer> = ExchangeWsStream<WebSocketSerdeParser, Transformer>;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BitstampSpot;

impl Connector for BitstampSpot {
    const ID: ExchangeId = ExchangeId::Bitstamp;
    type Channel = BitstampChannel;
    type Market = BitstampMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = BitstampSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(WEBSOCKET_BASE_URL_BITSTAMP).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        let messages = exchange_subs
            .into_iter()
            .map(|sub| {
                let channel = format!("{}{}", sub.channel.as_ref(), sub.market.as_ref(),);

                WsMessage::text(
                    serde_json::json!({
                        "event": "bts:subscribe",
                        "data": {
                            "channel": channel
                        }
                    })
                    .to_string(),
                )
            })
            .collect::<Vec<_>>();

        messages
    }
}
