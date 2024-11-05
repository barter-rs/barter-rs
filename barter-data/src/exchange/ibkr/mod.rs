use self::{
    channel::IbkrChannel, market::IbkrMarket, market_data::market_data_l1::IbkrMarketDataL1,
    subscriber::IbkrWebSocketSubscriber, unsolicited::account_updates::IbkrAccountResponse,
};
use crate::{
    exchange::{Connector, ExchangeId, ExchangeSub, PingInterval, StreamSelector},
    instrument::InstrumentData,
    subscription::{account::Accounts, book::OrderBooksL1, Map},
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream, NoInitialSnapshots,
};
use barter_integration::{
    error::SocketError,
    metric::Metric,
    protocol::{
        http::{
            public::PublicNoHeaders,
            rest::{client::RestClient, RestRequest},
            HttpParser,
        },
        websocket::WsMessage,
    },
};
use barter_macro::{DeExchange, SerExchange};
use reqwest::{header::HeaderMap, Error, StatusCode};
use serde::Deserialize;
use std::{borrow::Cow, time::Duration};
use subscription::IbkrPlatformEvent;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use url::Url;
use validator::IbkrWebSocketSubValidator;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific channel used for generating [`Connector::requests`].
pub mod channel;

/// Defines the type that translates a Barter [`Subscription`](crate::subscription::Subscription)
/// into an exchange [`Connector`] specific market used for generating [`Connector::requests`].
pub mod market;

pub mod market_data;

pub mod subscriber;

/// [`Subscription`](crate::subscription::Subscription) response type and response
/// [`Validator`](barter_integration::Validator) for [`Ibkr`].
pub mod subscription;

// /// Public trade types for [`Ibkr`].
// pub mod trade;

pub mod unsolicited;

/// Custom [`SubscriptionValidator`](crate::subscriber::validator::SubscriptionValidator)
/// implementation for [`Ibkr`].
pub mod validator;

/// [`Ibkr`] server base url for websockets.
///
/// See docs: <https://interactivebrokers.github.io/cpwebapi/websockets>
pub const BASE_URL_IBKR_WS: &str = "wss://localhost:5000/v1/api/ws";

/// [`Ibkr`] server base url for endpoints.
///
/// See docs: <https://interactivebrokers.github.io/cpwebapi/endpoints>
pub const BASE_URL_IBKR_REST: &str = "https://localhost:5000/v1/api";

/// [`Ibkr`] server [`PingInterval`] duration.
///
/// See docs: <https://interactivebrokers.github.io/cpwebapi/use-cases#session-duration>
pub const PING_INTERVAL_IBKR: Duration = Duration::from_secs(60);

/// [`Ibkr`] exchange.
///
/// See docs: <https://interactivebrokers.github.io/cpwebapi/websockets>
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct Ibkr;

impl Connector for Ibkr {
    const ID: ExchangeId = ExchangeId::Ibkr;
    type Channel = IbkrChannel;
    type Market = IbkrMarket;
    type Subscriber = IbkrWebSocketSubscriber;
    type SubValidator = IbkrWebSocketSubValidator;
    type SubResponse = IbkrPlatformEvent;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_IBKR_WS).map_err(SocketError::UrlParse)
    }

    fn ping_interval() -> Option<PingInterval> {
        Some(PingInterval {
            interval: tokio::time::interval(PING_INTERVAL_IBKR),
            ping: || WsMessage::text("tic"), // tic = tickle, ibkr terminology
        })
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        exchange_subs
            .into_iter()
            .map(|sub| sub.message())
            .collect::<Vec<WsMessage>>()
    }

    fn expected_responses<InstrumentKey>(map: &Map<InstrumentKey>) -> usize {
        map.0.len() + 3 // the count of subscriptions in the map + (system, status, account) messages
    }
}

impl ExchangeSub<IbkrChannel, IbkrMarket> {
    fn message(self) -> WsMessage {
        let msg = format!(
            r#"s{}+{}+{{"fields":[{}]}}"#,
            self.channel.sub_type, self.market.contract_id, self.market.fields,
        );
        WsMessage::Text(msg)
    }
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL1> for Ibkr
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, OrderBooksL1, IbkrMarketDataL1>,
    >;
}

impl<Instrument> StreamSelector<Instrument, Accounts> for Ibkr
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = ExchangeWsStream<
        StatelessTransformer<Self, Instrument::Key, Accounts, IbkrAccountResponse>,
    >;
}

#[derive(Debug)]
pub struct IbkrRest {
    /// HTTP [`reqwest::Client`]
    pub http_client: reqwest::Client,

    /// metric sender channel
    pub http_metric_tx: UnboundedSender<Metric>,

    /// metric receiver channel
    pub http_metric_rx: UnboundedReceiver<Metric>,
}

impl IbkrRest {
    pub fn new() -> Result<Self, Error> {
        // Construct Metric channel to send Http execution metrics over
        let (http_metric_tx, http_metric_rx) = mpsc::unbounded_channel();

        // construct http client without validating TLS
        // Ibkr API is a proxy at localhost, so may not have proper certs set up
        let mut default_headers = HeaderMap::new();
        // https://stackoverflow.com/a/69027259
        default_headers.insert("user-agent", "barter-rs".parse().unwrap());
        let http_client = reqwest::Client::builder()
            .default_headers(default_headers)
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self {
            http_client,
            http_metric_tx,
            http_metric_rx,
        })
    }

    async fn get_session(self) -> Result<String, SocketError> {
        let rest_client = RestClient {
            http_client: self.http_client,
            base_url: Cow::Borrowed(BASE_URL_IBKR_REST),
            strategy: PublicNoHeaders {},
            parser: IbkrTickleParser,
        };

        let (response, _metric) = rest_client.execute(IbkrTickleRequest).await?;
        Ok(response.session)
    }
}

struct IbkrTickleParser;

impl HttpParser for IbkrTickleParser {
    type ApiError = serde_json::Value;
    type OutputError = barter_integration::error::SocketError;

    fn parse_api_error(&self, status: StatusCode, api_error: Self::ApiError) -> Self::OutputError {
        // For simplicity, use serde_json::Value as Error and extract raw String for parsing
        let error = api_error.to_string();

        // Parse Ftx error message to determine custom ExecutionError variant
        match error.as_str() {
            message if message.contains("Invalid login credentials") => {
                SocketError::HttpUnauthorized(error)
            }
            _ => SocketError::HttpResponse(status, error),
        }
    }
}

struct IbkrTickleRequest;

impl RestRequest for IbkrTickleRequest {
    type Response = IbkrTickleResponse;
    type QueryParams = ();
    type Body = ();

    fn path(&self) -> Cow<'static, str> {
        Cow::Borrowed("/tickle")
    }

    fn method() -> reqwest::Method {
        reqwest::Method::POST
    }
}

#[derive(Deserialize)]
struct IbkrTickleResponse {
    session: String,
}

#[derive(Debug)]
pub struct IbkrWebSocketRequest {
    url: Url,
    session_id: String,
}

impl IbkrWebSocketRequest {
    fn new(url: Url, session_id: String) -> Self {
        Self { url, session_id }
    }

    fn request(self) -> tokio_tungstenite::tungstenite::http::Request<()> {
        // Ibkr websocket subscriptions require a session ID header
        let url = self.url.as_str();
        let mut req = url.into_client_request().unwrap();
        let headers = req.headers_mut();
        headers.insert("cookie", format!("api={}", self.session_id).parse().unwrap());
        // debug!("headers: {:?}", headers);
        req
    }
}
