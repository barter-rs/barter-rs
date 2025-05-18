use self::{
    channel::OneTradingChannel, market::OneTradingMarket, subscription::OneTradingResponse, trade::OneTradingTrade,
};
use crate::{
    ExchangeWsStream, NoInitialSnapshots,
    exchange::{Connector, ExchangeSub, PingInterval, StreamSelector},
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::{
        Map, 
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use barter_macro::{DeExchange, SerExchange};
use derive_more::Display;
use serde_json::json;
use std::time::Duration;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// Generic [`OneTradingPayload<T>`](message::OneTradingPayload) type for OneTrading messages
pub mod message;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](barter_integration::Validator) for OneTrading.
pub mod subscription;

/// Public trade types for OneTrading.
pub mod trade;

/// Orderbook types for OneTrading.
pub mod book;

/// [`OneTrading`] server base url.
pub const BASE_URL_ONETRADING: &str = "wss://streams.onetrading.com/ws";

/// [`OneTrading`] server [`PingInterval`] duration.
pub const PING_INTERVAL_ONETRADING: Duration = Duration::from_secs(30);

/// [`OneTrading`] execution.
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
pub struct OneTrading;

impl Connector for OneTrading {
    const ID: ExchangeId = ExchangeId::OneTrading;
    type Channel = OneTradingChannel;
    type Market = OneTradingMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = OneTradingResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_ONETRADING).map_err(SocketError::UrlParse)
    }

    fn ping_interval() -> Option<PingInterval> {
        Some(PingInterval {
            interval: tokio::time::interval(PING_INTERVAL_ONETRADING),
            ping: || {
                WsMessage::text(
                    json!({
                        "type": "PING"
                    })
                    .to_string(),
                )
            },
        })
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        // Group subscriptions by channel to properly format for OneTrading API
        let mut channels = Vec::new();
        
        for sub in exchange_subs {
            let channel_obj = json!({
                "name": sub.channel.as_ref(),
                "instrument": sub.market.as_ref()
            });
            
            channels.push(channel_obj);
        }
        
        vec![WsMessage::text(
            json!({
                "type": "SUBSCRIBE",
                "channels": channels
            })
            .to_string(),
        )]
    }

    fn expected_responses<InstrumentKey>(_: &Map<InstrumentKey>) -> usize {
        // OneTrading returns a single subscription response for all channels
        1
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for OneTrading
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, OneTradingTrade>>;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL1> for OneTrading
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBooksL1, book::OneTradingOrderBookL1Message>,
    >;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for OneTrading
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBooksL2, book::OneTradingOrderBookL2Message>,
    >;
}