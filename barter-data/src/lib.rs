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
//! - [`StreamBuilder`](streams::builder::StreamBuilder) for initialising [`MarketStream`]s of specific data kinds.
//! - [`DynamicStreams`](streams::builder::dynamic::DynamicStreams) for initialising [`MarketStream`]s of every supported data kind at once.
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
//!         .with_error_handler(|error| warn!(?error, "MarketStream generated error"));
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
use async_trait::async_trait;
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    Transformer,
    error::SocketError,
    protocol::{
        StreamParser,
        websocket::{WsError, WsMessage, WsSink, WsStream},
    },
    stream::ExchangeStream,
};
use futures::{SinkExt, Stream, StreamExt};

use std::{collections::VecDeque, future::Future};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

/// All [`Error`](std::error::Error)s generated in Barter-Data.
pub mod error;

/// Defines the generic [`MarketEvent<T>`](MarketEvent) used in every [`MarketStream`].
pub mod event;

/// [`Connector`] implementations for each exchange.
pub mod exchange;

/// High-level API types used for building [`MarketStream`]s from collections
/// of Barter [`Subscription`]s.
pub mod streams;

/// [`Subscriber`], [`SubscriptionMapper`](subscriber::mapper::SubscriptionMapper) and
/// [`SubscriptionValidator`](subscriber::validator::SubscriptionValidator)  traits that define how a
/// [`Connector`] will subscribe to exchange [`MarketStream`]s.
///
/// Standard implementations for subscribing to WebSocket [`MarketStream`]s are included.
pub mod subscriber;

/// Types that communicate the type of each [`MarketStream`] to initialise, and what normalised
/// Barter output type the exchange will be transformed into.
pub mod subscription;

/// [`InstrumentData`] trait for instrument describing data.
pub mod instrument;

/// [`OrderBook`](books::OrderBook) related types, and utilities for initialising and maintaining
/// a collection of sorted local Instrument [`OrderBook`](books::OrderBook)s
pub mod books;

/// Generic [`ExchangeTransformer`] implementations used by [`MarketStream`]s to translate exchange
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

/// Convenient type alias for an [`ExchangeStream`] utilizing a tungstenite
/// [`WebSocket`](barter_integration::protocol::websocket::WebSocket).
pub type ExchangeWsStream<Parser, Transformer> = ExchangeStream<Parser, WsStream, Transformer>;

/// Defines a generic identification type for the implementor.
pub trait Identifier<T> {
    fn id(&self) -> T;
}

/// [`Stream`] that yields [`Market<Kind>`](MarketEvent) events. The type of [`Market<Kind>`](MarketEvent)
/// depends on the provided [`SubscriptionKind`] of the passed [`Subscription`]s.
#[async_trait]
pub trait MarketStream<Exchange, Instrument, Kind>
where
    Self: Stream<Item = Result<MarketEvent<Instrument::Key, Kind::Event>, DataError>>
        + Send
        + Sized
        + Unpin,
    Exchange: Connector,
    Instrument: InstrumentData,
    Kind: SubscriptionKind,
{
    async fn init<SnapFetcher>(
        subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> Result<Self, DataError>
    where
        SnapFetcher: SnapshotFetcher<Exchange, Kind>,
        Subscription<Exchange, Instrument, Kind>:
            Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
}

/// Defines how to fetch market data snapshots for a collection of [`Subscription`]s.
///
/// Useful when a [`MarketStream`] requires an initial snapshot on start-up.
///
/// See examples such as Binance OrderBooksL2: <br>
/// - [`BinanceSpotOrderBooksL2SnapshotFetcher`](exchange::binance::spot::l2::BinanceSpotOrderBooksL2SnapshotFetcher)
/// - [`BinanceFuturesUsdOrderBooksL2SnapshotFetcher`](exchange::binance::futures::l2::BinanceFuturesUsdOrderBooksL2SnapshotFetcher)
pub trait SnapshotFetcher<Exchange, Kind> {
    fn fetch_snapshots<Instrument>(
        subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, Kind::Event>>, SocketError>> + Send
    where
        Exchange: Connector,
        Instrument: InstrumentData,
        Kind: SubscriptionKind,
        Kind::Event: Send,
        Subscription<Exchange, Instrument, Kind>: Identifier<Exchange::Market>;
}

#[async_trait]
impl<Exchange, Instrument, Kind, Transformer, Parser> MarketStream<Exchange, Instrument, Kind>
    for ExchangeWsStream<Parser, Transformer>
where
    Exchange: Connector + Send + Sync,
    Instrument: InstrumentData,
    Kind: SubscriptionKind + Send + Sync,
    Transformer: ExchangeTransformer<Exchange, Instrument::Key, Kind> + Send,
    Kind::Event: Send,
    Parser: StreamParser<Transformer::Input, Message = WsMessage, Error = WsError> + Send,
{
    async fn init<SnapFetcher>(
        subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> Result<Self, DataError>
    where
        SnapFetcher: SnapshotFetcher<Exchange, Kind>,
        Subscription<Exchange, Instrument, Kind>:
            Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
    {
        // Connect & subscribe
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
            Exchange::ID,
            ws_sink,
            ws_sink_rx,
        ));

        // Spawn optional task to distribute custom application-level pings to the exchange
        if let Some(ping_interval) = Exchange::ping_interval() {
            tokio::spawn(schedule_pings_to_exchange(
                Exchange::ID,
                ws_sink_tx.clone(),
                ping_interval,
            ));
        }

        // Initialise Transformer associated with this Exchange and SubscriptionKind
        let mut transformer =
            Transformer::init(instrument_map, &initial_snapshots, ws_sink_tx).await?;

        // Process any buffered active subscription events received during Subscription validation
        let mut processed = process_buffered_events::<Parser, Transformer>(
            &mut transformer,
            buffered_websocket_events,
        );

        // Extend buffered events with any initial snapshot events
        processed.extend(initial_snapshots.into_iter().map(Ok));

        Ok(ExchangeWsStream::new(ws_stream, transformer, processed))
    }
}

/// Implementation of [`SnapshotFetcher`] that does not fetch any initial market data snapshots.
/// Often used for stateless [`MarketStream`]s, such as public trades.
#[derive(Debug)]
pub struct NoInitialSnapshots;

impl<Exchange, Kind> SnapshotFetcher<Exchange, Kind> for NoInitialSnapshots {
    fn fetch_snapshots<Instrument>(
        _: &[Subscription<Exchange, Instrument, Kind>],
    ) -> impl Future<Output = Result<Vec<MarketEvent<Instrument::Key, Kind::Event>>, SocketError>> + Send
    where
        Exchange: Connector,
        Instrument: InstrumentData,
        Kind: SubscriptionKind,
        Kind::Event: Send,
        Subscription<Exchange, Instrument, Kind>: Identifier<Exchange::Market>,
    {
        std::future::ready(Ok(vec![]))
    }
}

pub fn process_buffered_events<Parser, StreamTransformer>(
    transformer: &mut StreamTransformer,
    events: Vec<Parser::Message>,
) -> VecDeque<Result<StreamTransformer::Output, StreamTransformer::Error>>
where
    Parser: StreamParser<StreamTransformer::Input>,
    StreamTransformer: Transformer,
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
