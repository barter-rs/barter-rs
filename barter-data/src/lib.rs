#![forbid(unsafe_code)]
#![warn(clippy::all)]
#![allow(clippy::pedantic, clippy::type_complexity)]
#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms
)]

//! # Barter-Data
//! A high-performance WebSocket integration library for streaming public market data from leading cryptocurrency
//! exchanges - batteries included. It is:
//! * **Easy**: Barter-Data's simple [`StreamBuilder`](streams::builder::StreamBuilder) and [`DynamicStreams`](streams::builder::DynamicStreams) interface allows for easy & quick setup (see example below and /examples!).
//! * **Normalised**: Barter-Data's unified interface for consuming public WebSocket data means every Exchange returns a normalised data model.
//! * **Real-Time**: Barter-Data utilises real-time WebSocket integrations enabling the consumption of normalised tick-by-tick data.
//! * **Extensible**: Barter-Data is highly extensible, and therefore easy to contribute to with coding new integrations!
//!
//! ## User API
//! - [`StreamBuilder`](streams::builder::StreamBuilder) for initialising [`MarketStream`]s of specific data kinds.
//! - [`DynamicStreams`](streams::builder::DynamicStreams) for initialising [`MarketStream`]s of every supported data kind at once.
//! - Define what exchange market data you want to stream using the [`Subscription`] type.
//! - Pass [`Subscription`]s to the [`StreamBuilder::subscribe`](streams::builder::StreamBuilder::subscribe) or [`DynamicStreams::init`](streams::builder::DynamicStreams::init) methods.
//! - Each call to the [`StreamBuilder::subscribe`](streams::builder::StreamBuilder::subscribe) (or each batch passed to the [`DynamicStreams::init`](streams::builder::DynamicStreams::init))
//!   method opens a new WebSocket connection to the exchange - giving you full control.
//!
//! ## Examples
//! For a comprehensive collection of examples, see the /examples directory.
//!
//! ### Multi Exchange Public Trades
//! ```rust,no_run
//! use barter_data::exchange::gateio::spot::GateioSpot;
//! use barter_data::{
//!     exchange::{
//!         binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
//!         coinbase::Coinbase,
//!         okx::Okx,
//!     },
//!     streams::Streams,
//!     subscription::trade::PublicTrades,
//! };
//! use barter_integration::model::instrument::kind::InstrumentKind;
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Initialise PublicTrades Streams for various exchanges
//!     // '--> each call to StreamBuilder::subscribe() initialises a separate WebSocket connection
//!     let streams = Streams::<PublicTrades>::builder()
//!         .subscribe([
//!             (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
//!             (BinanceSpot::default(), "eth", "usdt", InstrumentKind::Spot, PublicTrades),
//!         ])
//!         .subscribe([
//!             (BinanceFuturesUsd::default(), "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
//!             (BinanceFuturesUsd::default(), "eth", "usdt", InstrumentKind::Perpetual, PublicTrades),
//!         ])
//!         .subscribe([
//!             (Coinbase, "btc", "usd", InstrumentKind::Spot, PublicTrades),
//!             (Coinbase, "eth", "usd", InstrumentKind::Spot, PublicTrades),
//!         ])
//!         .subscribe([
//!             (GateioSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
//!             (GateioSpot::default(), "eth", "usdt", InstrumentKind::Spot, PublicTrades),
//!         ])
//!         .subscribe([
//!             (Okx, "btc", "usdt", InstrumentKind::Spot, PublicTrades),
//!             (Okx, "eth", "usdt", InstrumentKind::Spot, PublicTrades),
//!             (Okx, "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
//!             (Okx, "eth", "usdt", InstrumentKind::Perpetual, PublicTrades),
//!        ])
//!         .init()
//!         .await
//!         .unwrap();
//!
//!     // Join all exchange PublicTrades streams into a single tokio_stream::StreamMap
//!     // Notes:
//!     //  - Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
//!     //  - Use `streams.join()` to join all exchange streams into a single mpsc::UnboundedReceiver!
//!     let mut joined_stream = streams.join_map().await;
//!
//!     while let Some((exchange, trade)) = joined_stream.next().await {
//!         println!("Exchange: {exchange}, Market<PublicTrade>: {trade:?}");
//!     }
//! }
//! ```

use crate::{
    error::DataError,
    event::MarketEvent,
    exchange::{Connector, ExchangeId, PingInterval},
    instrument::InstrumentData,
    subscriber::Subscriber,
    subscription::{Subscription, SubscriptionKind},
    transformer::ExchangeTransformer,
};
use async_trait::async_trait;
use barter_integration::{
    protocol::websocket::{WebSocketParser, WsMessage, WsSink, WsStream},
    ExchangeStream,
};
use futures::{SinkExt, Stream, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error};

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

/// Generic [`ExchangeTransformer`] implementations used by [`MarketStream`]s to translate exchange
/// specific types to normalised Barter types.
///
/// Standard implementations that work for most exchanges are included such as: <br>
/// - [`StatelessTransformer`](transformer::stateless::StatelessTransformer) for
///   [`PublicTrades`](subscription::trade::PublicTrades)
///   and [`OrderBooksL1`](subscription::book::OrderBooksL1) streams. <br>
/// - [`MultiBookTransformer`](transformer::book::MultiBookTransformer) for
///   [`OrderBooksL2`](subscription::book::OrderBooksL2) and
///   [`OrderBooksL3`](subscription::book::OrderBooksL3) streams.
pub mod transformer;

/// Convenient type alias for an [`ExchangeStream`] utilising a tungstenite
/// [`WebSocket`](barter_integration::protocol::websocket::WebSocket).
pub type ExchangeWsStream<Transformer> = ExchangeStream<WebSocketParser, WsStream, Transformer>;

/// Defines a generic identification type for the implementor.
pub trait Identifier<T> {
    fn id(&self) -> T;
}

/// [`Stream`] that yields [`Market<Kind>`](MarketEvent) events. The type of [`Market<Kind>`](MarketEvent)
/// depends on the provided [`SubscriptionKind`] of the passed [`Subscription`]s.
#[async_trait]
pub trait MarketStream<Exchange, Instrument, Kind>
where
    Self: Stream<Item = Result<MarketEvent<Instrument::Id, Kind::Event>, DataError>>
        + Send
        + Sized
        + Unpin,
    Exchange: Connector,
    Instrument: InstrumentData,
    Kind: SubscriptionKind,
{
    async fn init(
        subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> Result<Self, DataError>
    where
        Subscription<Exchange, Instrument, Kind>:
            Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
}

#[async_trait]
impl<Exchange, Instrument, Kind, Transformer> MarketStream<Exchange, Instrument, Kind>
    for ExchangeWsStream<Transformer>
where
    Exchange: Connector + Send + Sync,
    Instrument: InstrumentData,
    Kind: SubscriptionKind + Send + Sync,
    Transformer: ExchangeTransformer<Exchange, Instrument::Id, Kind> + Send,
    Kind::Event: Send,
{
    async fn init(
        subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> Result<Self, DataError>
    where
        Subscription<Exchange, Instrument, Kind>:
            Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
    {
        // Connect & subscribe
        let (websocket, map) = Exchange::Subscriber::subscribe(subscriptions).await?;

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

        // Construct Transformer associated with this Exchange and SubscriptionKind
        let transformer = Transformer::new(ws_sink_tx, map).await?;

        Ok(ExchangeWsStream::new(ws_stream, transformer))
    }
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
                "failed to send  output message to the exchange via WsSink"
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
