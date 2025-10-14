use self::{
    channel::MexcChannel,
    market::MexcMarket,
    subscription::{MexcAggInterval, MexcWsMethod, MexcWsSub},
    validator::MexcWebSocketSubValidator,
};
use crate::{
    ExchangeWsPbStream, Identifier, NoInitialSnapshots,
    exchange::{Connector, ExchangeSub, PingInterval, StreamSelector},
    instrument::InstrumentData,
    subscriber::WebSocketSubscriber,
    subscription::{Map, book::OrderBooksL1, trade::PublicTrades},
    transformer::stateless::StatelessTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    error::SocketError, protocol::websocket::WsMessage, subscription::SubscriptionId,
};
use barter_macro::{DeExchange, SerExchange};
use derive_more::Display;
use serde::Deserialize;
use std::{
    borrow::Cow,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

pub mod book;
pub mod channel;
pub mod market;
pub mod subscription;
pub mod trade;
pub mod validator;

/// MEXC WebSocket API base URL for public market data streams (Secure).
/// Docs: <https://mexcdevelop.github.io/apidocs/spot_v3_en/#websocket-market-data>
pub const BASE_URL_MEXC: &str = "wss://wbs-api.mexc.com/ws";

/// [`Mexc`] exchange connector definition.
///
/// MEXC uses Protocol Buffers for its V3 WebSocket API for public data streams like trades.
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
pub struct Mexc;

impl Mexc {
    fn agg_interval_to_str(interval: MexcAggInterval) -> &'static str {
        match interval {
            MexcAggInterval::Ms10 => "10ms",
            MexcAggInterval::Ms100 => "100ms",
        }
    }
}

impl Connector for Mexc {
    const ID: ExchangeId = ExchangeId::Mexc;
    type Channel = MexcChannel;
    type Market = MexcMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = MexcWebSocketSubValidator;
    type SubResponse = self::subscription::MexcSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_MEXC).map_err(SocketError::UrlParse)
    }

    fn ping_interval() -> Option<PingInterval> {
        None
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        if exchange_subs.is_empty() {
            return Vec::new();
        }
        let default_interval = MexcAggInterval::default();

        let topics = exchange_subs
            .into_iter()
            .map(|sub| {
                format!(
                    "{}@{}@{}",
                    sub.channel.0,
                    Self::agg_interval_to_str(default_interval),
                    sub.market.0
                )
            })
            .collect::<Vec<String>>();

        let request_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let subscription_message = MexcWsSub {
            method: MexcWsMethod::Subscription,
            params: Cow::Owned(topics),
            id: request_id,
        };

        match serde_json::to_string(&subscription_message) {
            Ok(text_payload) => vec![WsMessage::Text(text_payload.into())],
            Err(e) => {
                eprintln!("Failed to serialize MEXC subscription request: {}", e);
                Vec::new()
            }
        }
    }

    fn expected_responses<InstrumentKey>(_: &Map<InstrumentKey>) -> usize {
        1
    }
}

// Stub `Deserialize` implementation to satisfy trait bounds.
// MEXC V3 uses Protocol Buffers, so proper decoding should be handled
// in the WebSocket layer rather than via Serde text deserialisation.
impl<'de> Deserialize<'de> for self::trade::proto::PushDataV3ApiWrapper {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "Attempted to Deserialize PushDataV3ApiWrapper with Serde text deserializer. \
            MEXC V3 uses Protobuf binary format. Implement proper Protobuf deserialization in the WebSocket handling layer.",
        ))
    }
}

impl Identifier<Option<SubscriptionId>> for self::trade::proto::PushDataV3ApiWrapper {
    fn id(&self) -> Option<SubscriptionId> {
        // Messages are tagged with a `channel` containing the base topic,
        // interval and symbol (eg. `spot@public.aggre.bookTicker.v3.api.pb@100ms@ETHUSDT`).
        // `SubscriptionId`s for `Mexc` streams are stored as
        // `"{base_channel}|{symbol}"`, so we parse the parts here to match.
        let mut parts = self.channel.rsplitn(3, '@');

        let symbol_from_channel = parts.next();
        let _interval = parts.next();
        let base_channel = parts.next();

        match (symbol_from_channel, base_channel) {
            (Some(symbol), Some(base)) => {
                let symbol = self.symbol.as_deref().unwrap_or(symbol);
                Some(SubscriptionId::from(format!("{base}|{symbol}")))
            }
            _ => Some(SubscriptionId::from(self.channel.as_str())),
        }
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Mexc
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsPbStream<
        StatelessTransformer<
            Self,
            Instrument::Key,
            PublicTrades,
            self::trade::proto::PushDataV3ApiWrapper,
        >,
    >;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL1> for Mexc
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsPbStream<
        StatelessTransformer<
            Self,
            Instrument::Key,
            OrderBooksL1,
            self::trade::proto::PushDataV3ApiWrapper,
        >,
    >;
}
