//! Exchange module for OKX. Implements all required traits and re-exports submodules.
use self::{
    channel::OkxChannel,
    futures::l2::{
        OkxFuturesOrderBooksL2SnapshotFetcher, OkxFuturesOrderBooksL2Transformer,
        OkxOrderBookL2 as OkxFuturesOrderBookL2,
    },
    market::OkxMarket,
    spot::l2::{
        OkxOrderBookL2 as OkxSpotOrderBookL2, OkxSpotOrderBooksL2SnapshotFetcher,
        OkxSpotOrderBooksL2Transformer,
    },
    subscription::OkxSubResponse,
    trade::OkxTrades,
};
use crate::{
    ExchangeWsStream, NoInitialSnapshots,
    exchange::{Connector, ExchangeSub, PingInterval, StreamSelector},
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

/// Defines the type that translates a Jackbot [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Jackbot [`Subscription`](crate::subscription::Subscription)
/// into an execution [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](jackbot_integration::Validator) for [`Okx`].
pub mod subscription;

/// Public trade types for [`Okx`].
pub mod trade;

/// [`Okx`] server base url.
///
/// See docs: <https://www.okx.com/docs-v5/en/#overview-api-resources-and-support>
pub const BASE_URL_OKX: &str = "wss://wsaws.okx.com:8443/ws/v5/public";

/// [`Okx`] server [`PingInterval`] duration.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-connect>
pub const PING_INTERVAL_OKX: Duration = Duration::from_secs(29);

/// [`Okx`] execution.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api>
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, DeExchange, SerExchange)]
pub struct Okx;

impl Connector for Okx {
    const ID: ExchangeId = ExchangeId::Okx;
    type Channel = OkxChannel;
    type Market = OkxMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = OkxSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_OKX).map_err(SocketError::UrlParse)
    }

    fn ping_interval() -> Option<PingInterval> {
        Some(PingInterval {
            interval: tokio::time::interval(PING_INTERVAL_OKX),
            ping: || WsMessage::text("ping"),
        })
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        vec![WsMessage::text(
            json!({
                "op": "subscribe",
                "args": &exchange_subs,
            })
            .to_string(),
        )]
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Okx
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Key, PublicTrades, OkxTrades>>;
}

/// Okx OrderBook L2 implementation
impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for Okx
where
    Instrument: InstrumentData,
{
    type SnapFetcher = OkxSpotOrderBooksL2SnapshotFetcher;
    type Stream = ExchangeWsStream<OkxSpotOrderBooksL2Transformer<Instrument::Key>>;
}

pub mod futures;
pub mod liquidation;
pub mod spot;
