use self::{
    channel::CoinbaseChannel, market::CoinbaseMarket, subscription::CoinbaseSubResponse,
    trade::CoinbaseTrade,
};
use crate::{
    Identifier, IdentifierStatic, NoInitialSnapshots, StreamSelector,
    error::DataError,
    event::MarketEvent,
    exchange::{Connector, ExchangeSub},
    init_ws_exchange_stream_with_initial_snapshots,
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::{Subscription, SubscriptionKind, trade::PublicTrades},
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{WebSocketSerdeParser, WsMessage},
};
use barter_macro::{DeExchange, SerExchange};
use derive_more::Display;
use futures_util::Stream;
use serde_json::json;
use std::future::Future;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`]
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Barter [`Subscription`]
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// [`Subscription`] response type and response
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

impl IdentifierStatic<ExchangeId> for Coinbase {
    fn id() -> ExchangeId {
        ExchangeId::Coinbase
    }
}

impl Connector for Coinbase {
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
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Coinbase
where
    Instrument: InstrumentData,
    Subscription<Self, Instrument, PublicTrades>:
        Identifier<CoinbaseChannel> + Identifier<CoinbaseMarket>,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, PublicTrades>>> + Send,
    ) -> impl Future<
        Output = Result<
            impl Stream<
                Item = Result<
                    MarketEvent<Instrument::Key, <PublicTrades as SubscriptionKind>::Event>,
                    DataError,
                >,
            >,
            DataError,
        >,
    > {
        init_ws_exchange_stream_with_initial_snapshots::<
            Self,
            Instrument,
            PublicTrades,
            WebSocketSerdeParser,
            StatelessTransformer<Self, Instrument::Key, PublicTrades, CoinbaseTrade>,
            NoInitialSnapshots,
        >(subscriptions)
    }
}
