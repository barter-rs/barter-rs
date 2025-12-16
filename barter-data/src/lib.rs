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
//! const STREAM_TIMEOUT: std::time::Duration = std::time::Duration::from_mins(1);
//!
//! #[tokio::main]
//! async fn main() {
//!     // Initialise PublicTrades Streams for various exchanges
//!     // '--> each call to StreamBuilder::subscribe() initialises a separate WebSocket connection
//!
//!     let streams = Streams::<PublicTrades>::builder()
//!         .subscribe(STREAM_TIMEOUT, [
//!             (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
//!             (BinanceSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
//!         ])
//!         .subscribe(STREAM_TIMEOUT, [
//!             (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
//!             (BinanceFuturesUsd::default(), "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
//!         ])
//!         .subscribe(STREAM_TIMEOUT, [
//!             (Coinbase, "btc", "usd", MarketDataInstrumentKind::Spot, PublicTrades),
//!             (Coinbase, "eth", "usd", MarketDataInstrumentKind::Spot, PublicTrades),
//!         ])
//!         .subscribe(STREAM_TIMEOUT, [
//!             (GateioSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
//!             (GateioSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
//!         ])
//!         .subscribe(STREAM_TIMEOUT, [
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
    Message, Transformer,
    error::SocketError,
    protocol::websocket::{AdminWs, WsMessage, WsParser, WsSink, process_admin_ws},
    serde::de::{Deserialiser, error::DeBinaryError},
    stream::ext::BarterStreamExt,
};
use bytes::Bytes;
use futures::{SinkExt, Stream, StreamExt};
use std::future::Future;
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
        stream_timeout: std::time::Duration,
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

pub async fn init_ws_exchange_stream<Exchange, Instrument, Kind, De, Transformer, SnapFetcher>(
    subscriptions: impl AsRef<Vec<Subscription<Exchange, Instrument, Kind>>>,
    timeout_stream: std::time::Duration,
) -> Result<
    impl Stream<Item = Result<MarketEvent<Instrument::Key, Kind::Event>, DataError>>,
    DataError,
>
where
    Exchange: Connector + IdentifierStatic<ExchangeId> + Send + Sync,
    Instrument: InstrumentData,
    Kind: SubscriptionKind + Send + Sync,
    Kind::Event: Send,
    De: Deserialiser<Bytes, Transformer::Input, Error = DeBinaryError>,
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

    // Construct channel to distribute Transformer messages (eg/ custom pongs) to the exchange
    let (ws_sink_tx, ws_sink_rx) = mpsc::unbounded_channel();

    // Split WebSocket into WsStream & WsSink components
    let (ws_sink, ws_stream) = websocket.split();

    // Spawn task to distribute Transformer messages (eg/ custom pongs) to the exchange
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
    let initial_updates = process_buffered_events::<De, _>(
        &mut transformer,
        buffered_websocket_events
            .into_iter()
            .map(WsMessage::into_data),
    )
    .collect::<Vec<_>>();

    // Define Stream Pipeline
    let ws_stream = ws_stream
        // Apply Stream timeout
        .with_timeout(timeout_stream, move || {
            warn!(
                timeout = ?timeout_stream,
                "stream ended due to consecutive event timeout"
            )
        })
        // Parse Result<WsMessage, WsError> -> Message<AdminWs, ExchangeTransformer::Input>
        .map(|ws_result| match WsParser::parse(ws_result) {
            Message::Admin(admin) => Message::Admin(admin),
            Message::Payload(payload) => De::deserialise(payload)
                .map(Message::Payload)
                .unwrap_or_else(|error| Message::Admin(AdminWs::DeError(error))),
        })
        // Apply ExchangeTransformer & flatten OutputIter
        .scan(transformer, |transformer, message| {
            use itertools::Either::*;

            let output = match message {
                Message::Admin(admin_ws) => match process_admin_ws(admin_ws) {
                    Ok(()) => None,
                    Err(error) => Some(Left(std::iter::once(Err(DataError::from(error))))),
                },
                Message::Payload(payload) => {
                    Some(Right(transformer.transform(payload).into_iter()))
                }
            };

            std::future::ready(Some(output))
        })
        .filter_map(std::future::ready)
        .flat_map(futures::stream::iter);

    let stream = futures::stream::iter(initial_updates)
        .chain(futures::stream::iter(initial_snapshots.into_iter().map(Ok)))
        .chain(ws_stream);

    Ok(stream)
}

pub fn process_buffered_events<De, StreamTransformer>(
    transformer: &mut StreamTransformer,
    events: impl IntoIterator<Item = Bytes>,
) -> impl Iterator<Item = Result<StreamTransformer::Output, StreamTransformer::Error>>
where
    De: Deserialiser<Bytes, StreamTransformer::Input>,
    De::Error: std::fmt::Debug,
    StreamTransformer: Transformer,
{
    events
        .into_iter()
        .filter_map(|input| match De::deserialise(input.clone()) {
            Ok(output) => Some(output),
            Err(error) => {
                let input_str =
                    String::from_utf8(input.to_vec()).unwrap_or_else(|error| error.to_string());
                warn!(
                    ?input,
                    %input_str,
                    ?error,
                    "failed to parse message buffered during Subscription validation"
                );
                None
            }
        })
        .flat_map(|parsed| transformer.transform(parsed))
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
