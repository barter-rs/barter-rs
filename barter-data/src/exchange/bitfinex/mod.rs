//!
//! ### Notes
//! #### SubscripionId
//! - Successful Bitfinex subscription responses contain a numeric `CHANNEL_ID` that must be used to
//!   identify future messages relating to that subscription (not persistent across connections).
//! - To identify the initial subscription response containing the `CHANNEL_ID`, the "channel" &
//!   "market" identifiers can be used for the `SubscriptionId(channel|market)`
//!   (eg/ SubscriptionId("trades|tBTCUSD")).
//! - Once the subscription has been validated and the `CHANNEL_ID` determined, each `SubscriptionId`
//!   in the `SubscriptionIds` `HashMap` is mutated to become `SubscriptionId(CHANNEL_ID)`.
//!   eg/ SubscriptionId("trades|tBTCUSD") -> SubscriptionId(69)
//!
//! #### Connection Limits
//! - The user is allowed up to 20 connections per minute on the public API.
//! - Each connection can be used to connect up to 25 different channels.
//!
//! #### Trade Variants
//! - Bitfinex trades subscriptions results in receiving tag="te" & tag="tu" trades.
//! - Both appear to be identical payloads, but "te" arriving marginally faster.
//! - Therefore, tag="tu" trades are filtered out and considered only as additional Heartbeats.

use self::{
    channel::BitfinexChannel, market::BitfinexMarket, message::BitfinexMessage,
    subscription::BitfinexPlatformEvent, validator::BitfinexWebSocketSubValidator,
};
use crate::{
    Identifier, IdentifierStatic, LiveMarketDataArgs, NoInitialSnapshots,
    error::DataError,
    event::MarketEvent,
    exchange::{Connector, ExchangeSub},
    init_ws_exchange_stream,
    instrument::InstrumentData,
    subscriber::WebSocketSubscriber,
    subscription::{
        Subscription,
        trade::{PublicTrade, PublicTrades},
    },
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    protocol::websocket::WsMessage, serde::de::DeJson, stream::data::DataStream,
};
use barter_macro::{DeExchange, SerExchange};
use derive_more::Display;
use futures_util::Stream;
use serde_json::json;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`]
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Barter [`Subscription`]
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// [`BitfinexMessage`] type for [`Bitfinex`].
pub mod message;

/// [`Subscription`] response types and response
/// [`Validator`](barter_integration::Validator) for [`Bitfinex`].
pub mod subscription;

/// Public trade types for [`Bitfinex`].
pub mod trade;

/// Custom `SubscriptionValidator` implementation for [`Bitfinex`].
pub mod validator;

/// [`Bitfinex`] server base url.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
pub const BASE_URL_BITFINEX: &str = "wss://api-pub.bitfinex.com/ws/2";

/// [`Bitfinex`] exchange.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
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
pub struct Bitfinex;

impl IdentifierStatic<ExchangeId> for Bitfinex {
    fn id() -> ExchangeId {
        ExchangeId::Bitfinex
    }
}

impl Connector for Bitfinex {
    type Channel = BitfinexChannel;
    type Market = BitfinexMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = BitfinexWebSocketSubValidator;
    type SubResponse = BitfinexPlatformEvent;

    fn url() -> Result<Url, url::ParseError> {
        Url::parse(BASE_URL_BITFINEX)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|ExchangeSub { channel, market }| {
                WsMessage::text(
                    json!({
                        "event": "subscribe",
                        "channel": channel.as_ref(),
                        "symbol": market.as_ref(),
                    })
                    .to_string(),
                )
            })
            .collect()
    }
}

impl<Instrument> DataStream<LiveMarketDataArgs<Self, Instrument, PublicTrades>> for Bitfinex
where
    Instrument: InstrumentData + 'static,
    Subscription<Self, Instrument, PublicTrades>: Identifier<BitfinexMarket>,
{
    type Item = Result<MarketEvent<Instrument::Key, PublicTrade>, DataError>;
    type Error = DataError;

    async fn init(
        args: LiveMarketDataArgs<Self, Instrument, PublicTrades>,
    ) -> Result<impl Stream<Item = Self::Item>, Self::Error> {
        init_ws_exchange_stream::<
            Self,
            Instrument,
            PublicTrades,
            DeJson,
            StatelessTransformer<Self, Instrument::Key, PublicTrades, BitfinexMessage>,
            NoInitialSnapshots,
        >(args)
        .await
    }
}
