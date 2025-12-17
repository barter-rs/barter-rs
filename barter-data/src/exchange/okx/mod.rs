use self::{
    channel::OkxChannel, market::OkxMarket, subscription::OkxSubResponse, trade::OkxTrades,
};
use crate::{
    Identifier, IdentifierStatic, LiveMarketDataArgs, NoInitialSnapshots,
    error::DataError,
    event::MarketEvent,
    exchange::{Connector, ExchangeSub, PingInterval},
    init_ws_exchange_stream,
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
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
use std::time::Duration;
use url::Url;

/// Defines the type that translates a Barter [`Subscription`]
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Barter [`Subscription`]
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

/// [`Subscription`] response type and response
/// [`Validator`](barter_integration::Validator) for [`Okx`].
pub mod subscription;

/// Public trade types for [`Okx`].
pub mod trade;

/// [`Okx`] server base url.
///
/// See docs: <https://www.okx.com/docs-v5/en/#overview-api-resources-and-support>
pub const BASE_URL_OKX: &str = "wss://ws.okx.com:8443/ws/v5/public";

/// [`Okx`] server [`PingInterval`] duration.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-connect>
pub const PING_INTERVAL_OKX: Duration = Duration::from_secs(29);

/// [`Okx`] exchange.
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api>
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
pub struct Okx;

impl IdentifierStatic<ExchangeId> for Okx {
    fn id() -> ExchangeId {
        ExchangeId::Okx
    }
}

impl Connector for Okx {
    type Channel = OkxChannel;
    type Market = OkxMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = OkxSubResponse;

    fn url() -> Result<Url, url::ParseError> {
        Url::parse(BASE_URL_OKX)
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

impl<Instrument> DataStream<LiveMarketDataArgs<Self, Instrument, PublicTrades>> for Okx
where
    Instrument: InstrumentData + 'static,
    Subscription<Self, Instrument, PublicTrades>: Identifier<OkxMarket>,
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
            StatelessTransformer<Self, Instrument::Key, PublicTrades, OkxTrades>,
            NoInitialSnapshots,
        >(args)
        .await
    }
}
