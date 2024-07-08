use self::subscription::ExchangeSub;
use crate::{
    instrument::InstrumentData,
    subscriber::{validator::SubscriptionValidator, Subscriber},
    subscription::{Map, SubKind, SubscriptionKind},
    MarketStream,
};
use barter_integration::{
    error::SocketError, model::instrument::kind::InstrumentKind, protocol::websocket::WsMessage,
    Validator,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    time::Duration,
};
use url::Url;

/// `BinanceSpot` & `BinanceFuturesUsd` [`Connector`] and [`StreamSelector`] implementations.
pub mod binance;

/// `Bitfinex` [`Connector`] and [`StreamSelector`] implementations.
pub mod bitfinex;

/// `Bitmex [`Connector`] and [`StreamSelector`] implementations.
pub mod bitmex;

/// `Bybit` ['Connector'] and ['StreamSelector'] implementation
pub mod bybit;

/// `Coinbase` [`Connector`] and [`StreamSelector`] implementations.
pub mod coinbase;

/// `GateioSpot`, `GateioFuturesUsd` & `GateioFuturesBtc` [`Connector`] and [`StreamSelector`]
/// implementations.
pub mod gateio;

/// `Kraken` [`Connector`] and [`StreamSelector`] implementations.
pub mod kraken;

/// `Okx` [`Connector`] and [`StreamSelector`] implementations.
pub mod okx;

/// Defines the generic [`ExchangeSub`] containing a market and channel combination used by an
/// exchange [`Connector`] to build [`WsMessage`] subscription payloads.
pub mod subscription;

/// Default [`Duration`] the [`Connector::SubValidator`] will wait to receive all success responses to actioned
/// [`Subscription`](subscription::Subscription) requests.
pub const DEFAULT_SUBSCRIPTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Defines the [`MarketStream`] kind associated with an exchange
/// [`Subscription`](subscription::Subscription) [`SubscriptionKind`].
///
/// ### Notes
/// Must be implemented by an exchange [`Connector`] if it supports a specific
/// [`SubscriptionKind`].
pub trait StreamSelector<Instrument, Kind>
where
    Self: Connector,
    Instrument: InstrumentData,
    Kind: SubscriptionKind,
{
    type Stream: MarketStream<Self, Instrument, Kind>;
}

/// Primary exchange abstraction. Defines how to translate Barter types into exchange specific
/// types, as well as connecting, subscribing, and interacting with the exchange server.
///
/// ### Notes
/// This must be implemented for a new exchange integration!
pub trait Connector
where
    Self: Clone + Default + Debug + for<'de> Deserialize<'de> + Serialize + Sized,
{
    /// Unique identifier for the exchange server being connected with.
    const ID: ExchangeId;

    /// Type that defines how to translate a Barter
    /// [`Subscription`](ubscription::Subscription) into an exchange specific channel
    /// to be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceChannel("@depth@100ms")`](binance::channel::BinanceChannel)
    /// - [`KrakenChannel("trade")`](kraken::channel::KrakenChannel)
    type Channel: AsRef<str>;

    /// Type that defines how to translate a Barter
    /// [`Subscription`](subscription::Subscription) into an exchange specific market that
    /// can be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceMarket("btcusdt")`](binance::market::BinanceMarket)
    /// - [`KrakenMarket("BTC/USDT")`](kraken::market::KrakenMarket)
    type Market: AsRef<str>;

    /// [`Subscriber`] type that establishes a connection with the exchange server, and actions
    /// [`Subscription`](subscription::Subscription)s over the socket.
    type Subscriber: Subscriber;

    /// [`SubscriptionValidator`] type that listens to responses from the exchange server and
    /// validates if the actioned [`Subscription`](subscription::Subscription)s were
    /// successful.
    type SubValidator: SubscriptionValidator;

    /// Deserialisable type that the [`Self::SubValidator`] expects to receive from the exchange server in
    /// response to the [`Subscription`](subscription::Subscription) [`Self::requests`]
    /// sent over the [`WebSocket`](barter_integration::protocol::websocket::WebSocket). Implements
    /// [`Validator`] in order to determine if [`Self`]
    /// communicates a successful [`Subscription`](subscription::Subscription) outcome.
    type SubResponse: Validator + Debug + DeserializeOwned;

    /// Base [`Url`] of the exchange server being connected with.
    fn url() -> Result<Url, SocketError>;

    /// Defines [`PingInterval`] of custom application-level
    /// [`WebSocket`](barter_integration::protocol::websocket::WebSocket) pings for the exchange
    /// server being connected with.
    ///
    /// Defaults to `None`, meaning that no custom pings are sent.
    fn ping_interval() -> Option<PingInterval> {
        None
    }

    /// Defines how to translate a collection of [`ExchangeSub`]s into the [`WsMessage`]
    /// subscription payloads sent to the exchange server.
    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage>;

    /// Number of [`Subscription`](subscription::Subscription) responses expected from the
    /// exchange server in responses to the requests send. Used to validate all
    /// [`Subscription`](subscription::Subscription)s were accepted.
    fn expected_responses<InstrumentId>(map: &Map<InstrumentId>) -> usize {
        map.0.len()
    }

    /// Expected [`Duration`] the [`SubscriptionValidator`] will wait to receive all success
    /// responses to actioned [`Subscription`](subscription::Subscription) requests.
    fn subscription_timeout() -> Duration {
        DEFAULT_SUBSCRIPTION_TIMEOUT
    }
}

/// Used when an exchange has servers different
/// [`InstrumentKind`] market data on distinct servers,
/// allowing all the [`Connector`] logic to be identical apart from what this trait provides.
///
/// ### Examples
/// - [`BinanceServerSpot`](binance::spot::BinanceServerSpot)
/// - [`BinanceServerFuturesUsd`](binance::futures::BinanceServerFuturesUsd)
pub trait ExchangeServer: Default + Debug + Clone + Send {
    const ID: ExchangeId;
    fn websocket_url() -> &'static str;
}

/// Defines the frequency and construction function for custom
/// [`WebSocket`](barter_integration::protocol::websocket::WebSocket) pings - used for exchanges
/// that require additional application-level pings.
#[derive(Debug)]
pub struct PingInterval {
    pub interval: tokio::time::Interval,
    pub ping: fn() -> WsMessage,
}

/// Unique identifier an exchange server [`Connector`].
///
/// ### Notes
/// An exchange may server different [`InstrumentKind`]
/// market data on distinct servers (eg/ Binance, Gateio). Such exchanges have multiple [`Self`]
/// variants, and often utilise the [`ExchangeServer`] trait.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename = "exchange", rename_all = "snake_case")]
pub enum ExchangeId {
    BinanceFuturesUsd,
    BinanceSpot,
    Bitfinex,
    Bitmex,
    BybitSpot,
    BybitPerpetualsUsd,
    Coinbase,
    GateioSpot,
    GateioFuturesUsd,
    GateioFuturesBtc,
    GateioPerpetualsBtc,
    GateioPerpetualsUsd,
    GateioOptions,
    Kraken,
    Okx,
}

impl From<ExchangeId> for barter_integration::model::Exchange {
    fn from(exchange_id: ExchangeId) -> Self {
        barter_integration::model::Exchange::from(exchange_id.as_str())
    }
}

impl Display for ExchangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ExchangeId {
    /// Return the &str representation of this [`ExchangeId`]
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::BinanceSpot => "binance_spot",
            ExchangeId::BinanceFuturesUsd => "binance_futures_usd",
            ExchangeId::Bitfinex => "bitfinex",
            ExchangeId::Bitmex => "bitmex",
            ExchangeId::BybitSpot => "bybit_spot",
            ExchangeId::BybitPerpetualsUsd => "bybit_perpetuals_usd",
            ExchangeId::Coinbase => "coinbase",
            ExchangeId::GateioSpot => "gateio_spot",
            ExchangeId::GateioFuturesUsd => "gateio_futures_usd",
            ExchangeId::GateioFuturesBtc => "gateio_futures_btc",
            ExchangeId::GateioPerpetualsUsd => "gateio_perpetuals_usd",
            ExchangeId::GateioPerpetualsBtc => "gateio_perpetuals_btc",
            ExchangeId::GateioOptions => "gateio_options",
            ExchangeId::Kraken => "kraken",
            ExchangeId::Okx => "okx",
        }
    }

    pub fn supports(&self, instrument_kind: InstrumentKind, sub_kind: SubKind) -> bool {
        use crate::subscription::SubKind::*;
        use ExchangeId::*;
        use InstrumentKind::*;

        match (self, instrument_kind, sub_kind) {
            (BinanceSpot, Spot, PublicTrades | OrderBooksL1) => true,
            (BinanceFuturesUsd, Perpetual, PublicTrades | OrderBooksL1 | Liquidations) => true,
            (Bitfinex, Spot, PublicTrades) => true,
            (Bitmex, Perpetual, PublicTrades) => true,
            (BybitSpot, Spot, PublicTrades) => true,
            (BybitPerpetualsUsd, Perpetual, PublicTrades) => true,
            (Coinbase, Spot, PublicTrades) => true,
            (GateioSpot, Spot, PublicTrades) => true,
            (GateioFuturesUsd, Future(_), PublicTrades) => true,
            (GateioFuturesBtc, Future(_), PublicTrades) => true,
            (GateioPerpetualsUsd, Perpetual, PublicTrades) => true,
            (GateioPerpetualsBtc, Perpetual, PublicTrades) => true,
            (GateioOptions, Option(_), PublicTrades) => true,
            (Kraken, Spot, PublicTrades | OrderBooksL1) => true,
            (Okx, Spot | Future(_) | Perpetual | Option(_), PublicTrades) => true,

            (_, _, _) => false,
        }
    }

    /// Determines whether the [`Connector`] associated with this [`ExchangeId`] supports the
    /// ingestion of market data for the provided [`InstrumentKind`].
    #[allow(clippy::match_like_matches_macro)]
    pub fn supports_instrument_kind(&self, instrument_kind: InstrumentKind) -> bool {
        use ExchangeId::*;
        use InstrumentKind::*;

        match (self, instrument_kind) {
            // Spot
            (
                BinanceFuturesUsd | Bitmex | BybitPerpetualsUsd | GateioPerpetualsUsd
                | GateioPerpetualsBtc,
                Spot,
            ) => false,
            (_, Spot) => true,

            // Future
            (GateioFuturesUsd | GateioFuturesBtc | Okx, Future(_)) => true,
            (_, Future(_)) => false,

            // Future Perpetual Swaps
            (
                BinanceFuturesUsd | Bitmex | Okx | BybitPerpetualsUsd | GateioPerpetualsUsd
                | GateioPerpetualsBtc,
                Perpetual,
            ) => true,
            (_, Perpetual) => false,

            // Option
            (GateioOptions | Okx, Option(_)) => true,
            (_, Option(_)) => false,
        }
    }
}
