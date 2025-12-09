#![forbid(unsafe_code)]
#![warn(
    unused,
    clippy::cognitive_complexity,
    unused_crate_dependencies,
    unused_extern_crates,
    clippy::unused_self,
    clippy::useless_let_if_seq,
    missing_debug_implementations,
    rust_2018_idioms,
    rust_2024_compatibility
)]
#![allow(clippy::type_complexity, clippy::too_many_arguments, type_alias_bounds)]

//! # Barter-Data
//! A high-performance WebSocket integration library for streaming public market data from leading cryptocurrency
//! exchanges - batteries included. It is:
//! * **Easy**: Barter-Data's simple [`StreamBuilder`](streams::builder::StreamBuilder) and [`DynamicStreams`](streams::builder::dynamic::DynamicStreams) interface allows for easy & quick setup (see example below and /examples!).
//! * **Normalised**: Barter-Data's unified interface for consuming public WebSocket data means every Exchange returns a normalised data model.
//! * **Real-Time**: Barter-Data utilises real-time WebSocket integrations enabling the consumption of normalised tick-by-tick data.

//! * **Extensible**: Barter-Data is highly extensible, and therefore easy to contribute to with coding new integrations!
//!
//! ## User API
//! - [`StreamBuilder`](streams::builder::StreamBuilder) for initialising market data streams of specific data kinds.
//! - [`DynamicStreams`](streams::builder::dynamic::DynamicStreams) for initialising market data streams of every supported data kind at once.
//! - Define what exchange market data you want to stream using the [`Subscription`] type.
//! - Pass [`Subscription`]s to the [`StreamBuilder::subscribe`](streams::builder::StreamBuilder::subscribe) or [`DynamicStreams::init`](streams::builder::dynamic::DynamicStreams::init) methods.
//! - Each call to the [`StreamBuilder::subscribe`](streams::builder::StreamBuilder::subscribe) (or each batch passed to the [`DynamicStreams::init`](streams::builder::dynamic::DynamicStreams::init))
//!   method opens a new WebSocket connection to the exchange - giving you full control.
//!
//! ## Examples
//! For a comprehensive collection of examples, see the /examples directory.
//!
//! ### Multi Exchange Public Trades
//! ```rust,no_run
//! use barter_data::{
//!     exchange::{
//!         gateio::spot::GateioSpot,
//!         binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
//!         coinbase::Coinbase,
//!         okx::Okx,
//!     },
//!     streams::{Streams, reconnect::stream::ReconnectingStream},
//!     subscription::trade::PublicTrades,
//! };
//! use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
//! use futures::StreamExt;
//! use tracing::warn;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Initialise PublicTrades Streams for various exchanges
//!     // '--> each call to StreamBuilder::subscribe() initialises a separate WebSocket connection
//!
//!     let streams = Streams::<PublicTrades>::builder()
//!         .subscribe([
//!             (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
//!             (BinanceSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
//!         ])
//!         .subscribe([
//!             (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
//!             (BinanceFuturesUsd::default(), "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
//!         ])
//!         .subscribe([
//!             (Coinbase, "btc", "usd", MarketDataInstrumentKind::Spot, PublicTrades),
//!             (Coinbase, "eth", "usd", MarketDataInstrumentKind::Spot, PublicTrades),
//!         ])
//!         .subscribe([
//!             (GateioSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
//!             (GateioSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
//!         ])
//!         .subscribe([
//!             (Okx, "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
//!             (Okx, "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
//!             (Okx, "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
//!             (Okx, "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
//!        ])
//!         .init()
//!         .await
//!         .unwrap();
//!
//!     // Select and merge every exchange Stream using futures_util::stream::select_all
//!     // Note: use `Streams.select(ExchangeId)` to interact with individual exchange streams!
//!     let mut joined_stream = streams
//!         .select_all()
//!         .with_error_handler(|error| warn!(?error, "stream generated error"));
//!
//!     while let Some(event) = joined_stream.next().await {
//!         println!("{event:?}");
//!     }
//! }
//! ```
use crate::{
    error::DataError,
    event::MarketEvent,
    exchange::{Connector, PingInterval},
    instrument::InstrumentData,
    subscriber::{Subscribed, Subscriber},
    subscription::{Subscription, SubscriptionKind},
    transformer::ExchangeTransformer,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    Message, MessageApp, Transformer, TransformerDeprecated,
    error::SocketError,
    protocol::{
        StreamParser,
        websocket::{WsError, WsMessage, WsSink, WsStream},
    },
    stream::ExchangeStream,
};
use futures::{Sink, SinkExt, Stream, StreamExt};

use crate::{
    exchange::{ApiMessage, AppMessage, binance::spot::BinanceSpot},
    subscription::Map,
};
use barter_instrument::{Keyed, index::error::IndexError};
use barter_integration::{
    protocol::websocket::init_websocket,
    serde::{DeBinaryError, DeTransformer, SeBinaryError, SeTransformer},
    socket::reconnecting::{
        ReconnectingSocket, backoff::ReconnectBackoff, init_reconnecting_socket,
        on_connect_err::ConnectErrorHandler, update::SocketUpdate,
    },
    stream::ext::indexed::Indexer,
    subscription::SubscriptionId,
};
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, future::Future, marker::PhantomData};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

/// All [`Error`](std::error::Error)s generated in Barter-Data.
pub mod error;

/// Defines the generic [`MarketEvent<T>`](MarketEvent) used in every market data stream.
pub mod event;

/// [`Connector`] implementations for each exchange.
pub mod exchange;

/// High-level API types used for building market data streams from collections
/// of Barter [`Subscription`]s.
pub mod streams;

/// [`Subscriber`], [`SubscriptionMapper`](subscriber::mapper::SubscriptionMapper) and
/// [`SubscriptionValidator`](subscriber::validator::SubscriptionValidator)  traits that define how a
/// [`Connector`] will subscribe to exchange market data streams.
///
/// Standard implementations for subscribing to WebSocket market data streams are included.
pub mod subscriber;

/// Types that communicate the type of each market data stream to initialise, and what normalised
/// Barter output type the exchange will be transformed into.
pub mod subscription;

/// [`InstrumentData`] trait for instrument describing data.
pub mod instrument;

/// [`OrderBook`](books::OrderBook) related types, and utilities for initialising and maintaining
/// a collection of sorted local Instrument [`OrderBook`](books::OrderBook)s
pub mod books;

/// Generic [`ExchangeTransformer`] implementations used by market data streams to translate exchange
/// specific types to normalised Barter types.
///
/// A standard [`StatelessTransformer`](transformer::stateless::StatelessTransformer) implementation
/// that works for most `Exchange`-`SubscriptionKind` combinations is included.
///
/// Cases that need custom logic, such as fetching initial [`OrderBooksL2`](subscription::book::OrderBooksL2)
/// and [`OrderBooksL3`](subscription::book::OrderBooksL3) snapshots on startup, may require custom
/// [`ExchangeTransformer`] implementations.
/// For examples, see [`Binance`](exchange::binance::Binance) [`OrderBooksL2`](subscription::book::OrderBooksL2)
/// [`ExchangeTransformer`] implementations for
/// [`spot`](exchange::binance::spot::l2::BinanceSpotOrderBooksL2Transformer) and
/// [`futures_usd`](exchange::binance::futures::l2::BinanceFuturesUsdOrderBooksL2Transformer).
pub mod transformer;

/// Defines a generic identification type for the implementor.
pub trait Identifier<T> {
    fn id(&self) -> T;
}

/// Defines a generic identification type for the implementor that is not dependent on state.
pub trait IdentifierStatic<T> {
    fn id() -> T;
}

/// Defines how to initialise a `Stream` of [`MarketEvent`] for the given subscriptions.
pub trait StreamSelector<Instrument, Kind>
where
    Self: Sized,
    Instrument: InstrumentData,
    Kind: SubscriptionKind,
{
    fn init(
        subscriptions: impl AsRef<Vec<Subscription<Self, Instrument, Kind>>> + Send,
    ) -> impl Future<
        Output = Result<
            impl Stream<Item = Result<MarketEvent<Instrument::Key, Kind::Event>, DataError>> + Send,
            DataError,
        >,
    > + Send;
}

/// Defines how to fetch market data snapshots for a collection of [`Subscription`]s.
///
/// Useful when a market data stream requires an initial snapshot on start-up.
///
/// See examples such as Binance OrderBooksL2: <br>
/// - [`BinanceSpotOrderBooksL2SnapshotFetcher`](exchange::binance::spot::l2::BinanceSpotOrderBooksL2SnapshotFetcher)
/// - [`BinanceFuturesUsdOrderBooksL2SnapshotFetcher`](exchange::binance::futures::l2::BinanceFuturesUsdOrderBooksL2SnapshotFetcher)
pub trait SnapshotFetcher<Exchange, Instrument, Kind>
where
    Instrument: InstrumentData,
    Kind: SubscriptionKind,
{
    fn fetch_snapshots(
        subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, Kind::Event>>, SocketError>> + Send;
}

/// Implementation of [`SnapshotFetcher`] that does not fetch any initial market data snapshots.
/// Often used for stateless market data streams, such as public trades.
#[derive(Debug)]
pub struct NoInitialSnapshots;

impl<Exchange, Instrument, Kind> SnapshotFetcher<Exchange, Instrument, Kind> for NoInitialSnapshots
where
    Instrument: InstrumentData,
    Kind: SubscriptionKind,
    Kind::Event: Send,
{
    fn fetch_snapshots(
        _: &[Subscription<Exchange, Instrument, Kind>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, Kind::Event>>, SocketError>> + Send
    {
        std::future::ready(Ok(vec![]))
    }
}

pub fn init_reconnecting_socket_with_updates<
    FnConnect,
    FnOnConnectErr,
    ConnectErr,
    Backoff,
    FnOnStTimeout,
    Socket,
    SinkItem,
    Admin,
    Payload,
>(
    connect: FnConnect,
    timeout_connect: std::time::Duration,
    on_connect_err: FnOnConnectErr,
    reconnect_backoff: Backoff,
    timeout_stream: std::time::Duration,
    on_stream_timeout: FnOnStTimeout,
) -> impl Stream<Item = SocketUpdate<impl Sink<SinkItem>, Socket::Item>>
where
    FnConnect: AsyncFnMut() -> Result<Socket, ConnectErr>,
    FnOnConnectErr: ConnectErrorHandler<ConnectErr>,
    Backoff: ReconnectBackoff,
    FnOnStTimeout: Fn() + 'static,
    Socket: Sink<SinkItem> + Stream<Item = Message<Admin, Payload>>,
{
    init_reconnecting_socket(connect, timeout_connect, reconnect_backoff)
        .on_connect_err(on_connect_err)
        // Todo: add timeouts before .with_socket_updates()
        .with_socket_updates()
}

// pub trait ServerSocket {
//     type Serialiser<'se>: Transformer<Self::OutMessage, Output<'se> = Result<bytes::Bytes, SeBinaryError>>;
//     type OutMessage;
//     type Deserialiser<'de>: Transformer<bytes::Bytes, Output<'de> = Result<Self::InMessage, DeBinaryError>>;
//     type InMessage;
//
//     fn base_url() -> &'static str;
// }
//
// pub async fn init_socket<Exchange, Instrument, Kind, SeTransf, OutMessage, DeTransf, InMessage>(
//     subscriptions: impl AsRef<Vec<Subscription<Exchange, Instrument, Kind>>>,
// ) -> Result<(), WsError>
// where
//     Exchange: ServerSocket,
// {
//     let socket = init_websocket::<
//         Exchange::Serialiser<'_>,
//         Exchange::OutMessage,
//         Exchange::Deserialiser<'_>,
//         Exchange::InMessage,
//     >(Exchange::base_url())
//     .await?;
//
//     Ok(())
// }

pub struct MessageKeyer<Key, Index, Message> {
    map: FnvHashMap<Key, Index>,
    phantom: PhantomData<Message>,
}

impl<Key, Index, Message> Indexer for MessageKeyer<Key, Index, Message>
where
    Index: Clone,
    Message: for<'a> Identifier<&'a Key>,
{
    type Unindexed = Message;
    type Indexed = Keyed<Index, Message>;

    type Error = String; // Todo: update this to less catch all type

    // Todo: Make Indexer more general:
    //  1. Move to barter-integration
    //  2. Make Error associated type or more general in some way
    //  3. How can we handle dynamic subscriptions?

    fn index(&self, item: Self::Unindexed) -> Result<Self::Indexed, Self::Error> {
        self.map
            .get(item.id())
            .map(|index| Keyed::new(index, item))
            .ok_or_else(|| "failed to find index for Unindexed key".to_string())
    }
}

impl<Key, Index, Message> FromIterator<(Key, Index)> for MessageKeyer<Key, Index, Message> {
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = (Key, Index)>,
    {
        let map = iter.into_iter().collect::<FnvHashMap<Key, Index>>();
        Self::new(map)
    }
}

impl<Key, Index, Message> MessageKeyer<Key, Index, Message> {
    pub fn new(map: FnvHashMap<Key, Index>) -> Self {
        Self {
            map,
            phantom: <_>::default(),
        }
    }
}

pub async fn init_binance_socket<Instrument, Kind>(
    url_custom: Option<url::Url>,
    subscriptions: impl AsRef<Vec<Subscription<BinanceSpot, Instrument, Kind>>>,
) -> Result<(), SocketError>
where
    Instrument: InstrumentData,
    Kind: SubscriptionKind,
    Subscription<BinanceSpot, Instrument, Kind>: Identifier<SubscriptionId>,
{
    let url = url_custom.unwrap_or(BinanceSpot::url()?);

    // Subscriptions are basically Requests...

    #[derive(Serialize)]
    pub struct BinanceRequest;
    #[derive(Deserialize)]
    pub struct BinanceMessage;

    let socket = init_websocket::<
        SeTransformer<BinanceRequest>,
        BinanceRequest,
        DeTransformer<BinanceMessage>,
        BinanceMessage,
    >(url.as_str())
    .await?;

    let subscriptions = subscriptions.as_ref();

    let keyer = MessageKeyer::from_iter(
        subscriptions
            .iter()
            .map(|sub| (sub.id(), sub.instrument.key())),
    );

    use barter_integration::stream::ext::StreamExt;

    let socket = socket.with_index(keyer);

    Ok(())
}

pub async fn init_ws_stream<Exchange, Instrument, Kind, FnDe, DeTransf, AppTransf, SnapFetcher>(
    subscriptions: impl AsRef<Vec<Subscription<Exchange, Instrument, Kind>>>,
) -> Result<impl Stream, WsError>
where
    Exchange: Connector + ApiMessage + AppMessage + IdentifierStatic<ExchangeId>,
    Instrument: InstrumentData,
    Kind: SubscriptionKind,
    DeTransf:
        for<'a> Transformer<bytes::Bytes, Output<'a> = Result<Exchange::Message, DeBinaryError>>,
    AppTransf: for<'a> Transformer<
            Exchange::Message,
            Output<'a> = MessageApp<Exchange::Response, Exchange::Payload>,
        >,
    SnapFetcher: SnapshotFetcher<Exchange, Instrument, Kind>,
{
    // Define variables for logging ergonomics
    let exchange = Exchange::id();
    let url = Exchange::url().unwrap();

    let mut socket = init_websocket(url).await?;
    // Todo: Subscribing & validating occurs here

    // Todo:
    //  - Should Ping, Pong, Respones be MessageAdmin::Application?
    //  - If we are doing dynamic subs (so no static SubMap), where do we Key Payloads?
    //     - Feels like we may need to hold off on applying AppTransf until context is present

    while let Some(message) = socket.next().await {
        match message {
            Message::Admin(admin) => {}
            Message::Payload(payload) => match payload {
                MessageApp::Ping => {}
                MessageApp::Pong => {}
                MessageApp::Response(_) => {}
                MessageApp::Payload(_) => {}
            },
        }
    }

    // Fetch any required initial MarketEvent snapshots
    let initial_snapshots = SnapFetcher::fetch_snapshots(subscriptions).await?;

    Ok(())
}

pub async fn init_ws_exchange_stream_with_initial_snapshots<
    Exchange,
    Instrument,
    Kind,
    Parser,
    Transformer,
    SnapFetcher,
>(
    subscriptions: impl AsRef<Vec<Subscription<Exchange, Instrument, Kind>>>,
) -> Result<ExchangeStream<Parser, WsStream, Transformer>, DataError>
where
    Exchange: Connector + IdentifierStatic<ExchangeId> + Send + Sync,
    Instrument: InstrumentData,
    Kind: SubscriptionKind + Send + Sync,
    Kind::Event: Send,
    Parser: StreamParser<Transformer::Input, Message = WsMessage, Error = WsError>,
    Transformer: ExchangeTransformer<Exchange, Instrument::Key, Kind>,
    SnapFetcher: SnapshotFetcher<Exchange, Instrument, Kind>,
    Subscription<Exchange, Instrument, Kind>:
        Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
{
    // Connect & subscribe
    let subscriptions = subscriptions.as_ref().as_slice();
    let Subscribed {
        websocket,
        map: instrument_map,
        buffered_websocket_events,
    } = Exchange::Subscriber::subscribe(subscriptions).await?;

    // Fetch any required initial MarketEvent snapshots
    let initial_snapshots = SnapFetcher::fetch_snapshots(subscriptions).await?;

    // Split WebSocket into WsStream & WsSink components
    let (ws_sink, ws_stream) = websocket.split();

    // Spawn task to distribute Transformer messages (eg/ custom pongs) to the exchange
    let (ws_sink_tx, ws_sink_rx) = mpsc::unbounded_channel();
    tokio::spawn(distribute_messages_to_exchange(
        Exchange::id(),
        ws_sink,
        ws_sink_rx,
    ));

    // Spawn optional task to distribute custom application-level pings to the exchange
    if let Some(ping_interval) = Exchange::ping_interval() {
        tokio::spawn(schedule_pings_to_exchange(
            Exchange::id(),
            ws_sink_tx.clone(),
            ping_interval,
        ));
    }

    // Initialise Transformer associated with this Exchange and SubscriptionKind
    let mut transformer = Transformer::init(instrument_map, &initial_snapshots, ws_sink_tx).await?;

    // Process any buffered active subscription events received during Subscription validation
    let mut processed =
        process_buffered_events::<Parser, Transformer>(&mut transformer, buffered_websocket_events);

    // Extend buffered events with any initial snapshot events
    processed.extend(initial_snapshots.into_iter().map(Ok));

    Ok(ExchangeStream::new(ws_stream, transformer, processed))
}

pub fn process_buffered_events<Parser, StreamTransformer>(
    transformer: &mut StreamTransformer,
    events: Vec<Parser::Message>,
) -> VecDeque<Result<StreamTransformer::Output, StreamTransformer::Error>>
where
    Parser: StreamParser<StreamTransformer::Input>,
    StreamTransformer: TransformerDeprecated,
{
    events
        .into_iter()
        .filter_map(|event| {
            Parser::parse(Ok(event))?
                .inspect_err(|error| {
                    warn!(
                        ?error,
                        "failed to parse message buffered during Subscription validation"
                    )
                })
                .ok()
        })
        .flat_map(|parsed| transformer.transform(parsed))
        .collect()
}

/// Transmit [`WsMessage`]s sent from the [`ExchangeTransformer`] to the exchange via
/// the [`WsSink`].
///
/// **Note:**
/// ExchangeTransformer is operating in a synchronous trait context so we use this separate task
/// to avoid adding `#[\async_trait\]` to the transformer - this avoids allocations.
pub async fn distribute_messages_to_exchange(
    exchange: ExchangeId,
    mut ws_sink: WsSink,
    mut ws_sink_rx: mpsc::UnboundedReceiver<WsMessage>,
) {
    while let Some(message) = ws_sink_rx.recv().await {
        if let Err(error) = ws_sink.send(message).await {
            if barter_integration::protocol::websocket::is_websocket_disconnected(&error) {
                break;
            }

            // Log error only if WsMessage failed to send over a connected WebSocket
            error!(
                %exchange,
                %error,
                "failed to send output message to the exchange via WsSink"
            );
        }
    }
}

/// Schedule the sending of custom application-level ping [`WsMessage`]s to the exchange using
/// the provided [`PingInterval`].
///
/// **Notes:**
///  - This is only used for those exchanges that require custom application-level pings.
///  - This is additional to the protocol-level pings already handled by `tokio_tungstenite`.
pub async fn schedule_pings_to_exchange(
    exchange: ExchangeId,
    ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
    PingInterval { mut interval, ping }: PingInterval,
) {
    loop {
        // Wait for next scheduled ping
        interval.tick().await;

        // Construct exchange custom application-level ping payload
        let payload = ping();
        debug!(%exchange, %payload, "sending custom application-level ping to exchange");

        if ws_sink_tx.send(payload).is_err() {
            break;
        }
    }
}

pub mod test_utils {
    use crate::{
        event::{DataKind, MarketEvent},
        subscription::trade::PublicTrade,
    };
    use barter_instrument::{Side, exchange::ExchangeId};
    use chrono::{DateTime, Utc};

    pub fn market_event_trade_buy<InstrumentKey>(
        time_exchange: DateTime<Utc>,
        time_received: DateTime<Utc>,
        instrument: InstrumentKey,
        price: f64,
        quantity: f64,
    ) -> MarketEvent<InstrumentKey, DataKind> {
        MarketEvent {
            time_exchange,
            time_received,
            exchange: ExchangeId::BinanceSpot,
            instrument,
            kind: DataKind::Trade(PublicTrade {
                id: "trade_id".to_string(),
                price,
                amount: quantity,
                side: Side::Buy,
            }),
        }
    }
}
