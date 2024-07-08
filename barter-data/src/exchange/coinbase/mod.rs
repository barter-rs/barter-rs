use self::{
    channel::CoinbaseChannel, market::CoinbaseMarket, subscription::CoinbaseSubResponse,
    trade::CoinbaseTrade,
};
use crate::{
    exchange::{Connector, ExchangeId, ExchangeSub, StreamSelector},
    instrument::InstrumentData,
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream,
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use barter_macro::{DeExchange, SerExchange};
use serde_json::json;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](barter_integration::Validator) for [`Coinbase`].
pub mod subscription;

/// Public trade types for [`Coinbase`].
pub mod trade;

/// [`Coinbase`] server base url.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
pub const BASE_URL_COINBASE: &str = "wss://ws-feed.exchange.coinbase.com";

/// [`Coinbase`] exchange.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview>
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
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

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::Text(
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
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Coinbase
where
    Instrument: InstrumentData,
{
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Id, PublicTrades, CoinbaseTrade>>;
}
