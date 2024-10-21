use super::Streams;
use crate::{
    error::DataError,
    exchange::StreamSelector,
    instrument::InstrumentData,
    streams::{
        consumer::{init_market_stream, MarketStreamResult, STREAM_RECONNECTION_POLICY},
        reconnect::stream::ReconnectingStream,
    },
    subscription::{Subscription, SubscriptionKind},
    Identifier,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::Validator;
use futures_util::StreamExt;
use std::{collections::HashMap, fmt::Debug, future::Future, pin::Pin};
use tokio::sync::mpsc;

/// Defines the [`MultiStreamBuilder`](multi::MultiStreamBuilder) API for ergonomically
/// initialising a common [`Streams<Output>`](Streams) from multiple
/// [`StreamBuilder<SubscriptionKind>`](StreamBuilder)s.
pub mod multi;

/// Defines the [`DynamicStreams`](dynamic::DynamicStreams) API for initialising an arbitrary number
/// of [`MarketStream`]s from the [`ExchangeId`] and [`SubKind`] enums, rather than concrete
/// types.
pub mod dynamic;

/// Communicative type alias representing the [`Future`] result of a [`Subscription`] [`validate`]
/// call generated whilst executing [`StreamBuilder::subscribe`].
pub type SubscribeFuture = Pin<Box<dyn Future<Output = Result<(), DataError>>>>;

/// Builder to configure and initialise a [`Streams<MarketEvent<SubscriptionKind::Event>`](Streams) instance
/// for a specific [`SubscriptionKind`].
#[derive(Default)]
pub struct StreamBuilder<InstrumentKey, Kind>
where
    Kind: SubscriptionKind,
{
    pub channels:
        HashMap<ExchangeId, ExchangeChannel<MarketStreamResult<InstrumentKey, Kind::Event>>>,
    pub futures: Vec<SubscribeFuture>,
}

impl<InstrumentKey, Kind> Debug for StreamBuilder<InstrumentKey, Kind>
where
    InstrumentKey: Debug,
    Kind: SubscriptionKind,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamBuilder<InstrumentKey, SubscriptionKind>")
            .field("channels", &self.channels)
            .field("num_futures", &self.futures.len())
            .finish()
    }
}

impl<InstrumentKey, Kind> StreamBuilder<InstrumentKey, Kind>
where
    Kind: SubscriptionKind,
{
    /// Construct a new [`Self`].
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            futures: Vec::new(),
        }
    }

    /// Add a collection of [`Subscription`]s to the [`StreamBuilder`] that will be actioned on
    /// a distinct [`WebSocket`](barter_integration::protocol::websocket::WebSocket) connection.
    ///
    /// Note that [`Subscription`]s are not actioned until the
    /// [`init()`](StreamBuilder::init()) method is invoked.
    pub fn subscribe<SubIter, Sub, Exchange, Instrument>(mut self, subscriptions: SubIter) -> Self
    where
        SubIter: IntoIterator<Item = Sub>,
        Sub: Into<Subscription<Exchange, Instrument, Kind>>,
        Exchange: StreamSelector<Instrument, Kind> + Ord + Send + Sync + 'static,
        Instrument: InstrumentData<Key = InstrumentKey> + Ord + 'static,
        Instrument::Key: Send + 'static,
        Kind: Ord + Send + Sync + 'static,
        Kind::Event: Send,
        Subscription<Exchange, Instrument, Kind>:
            Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
    {
        // Construct Vec<Subscriptions> from input SubIter
        let subscriptions = subscriptions.into_iter().map(Sub::into).collect::<Vec<_>>();

        // Acquire channel Sender to send Market<Kind::Event> from consumer loop to user
        // '--> Add ExchangeChannel Entry if this Exchange <--> SubscriptionKind combination is new
        let exchange_tx = self.channels.entry(Exchange::ID).or_default().tx.clone();

        // Add Future that once awaited will yield the Result<(), SocketError> of subscribing
        self.futures.push(Box::pin(async move {
            // Validate Subscriptions
            let mut subscriptions = subscriptions
                .into_iter()
                .map(Subscription::validate)
                .collect::<Result<Vec<_>, _>>()?;

            // Remove duplicate Subscriptions
            subscriptions.sort();
            subscriptions.dedup();

            // Initialise a MarketEvent `ReconnectingStream`
            init_market_stream(STREAM_RECONNECTION_POLICY, subscriptions)
                .await?
                .boxed()
                .forward_to(exchange_tx);

            Ok(())
        }));

        self
    }

    /// Spawn a [`MarketEvent<SubscriptionKind::Event>`](MarketEvent) consumer loop for each collection of
    /// [`Subscription`]s added to [`StreamBuilder`] via the
    /// [`subscribe()`](StreamBuilder::subscribe()) method.
    ///
    /// Each consumer loop distributes consumed [`MarketEvent<SubscriptionKind::Event>s`](MarketEvent) to
    /// the [`Streams`] `HashMap` returned by this method.
    pub async fn init(
        self,
    ) -> Result<Streams<MarketStreamResult<InstrumentKey, Kind::Event>>, DataError> {
        // Await Stream initialisation perpetual and ensure success
        futures::future::try_join_all(self.futures).await?;

        // Construct Streams using each ExchangeChannel receiver
        Ok(Streams {
            streams: self
                .channels
                .into_iter()
                .map(|(exchange, channel)| (exchange, channel.rx))
                .collect(),
        })
    }
}

/// Convenient type that holds the [`mpsc::UnboundedSender`] and [`mpsc::UnboundedReceiver`] for a
/// [`MarketEvent<T>`](MarketEvent) channel.
#[derive(Debug)]
pub struct ExchangeChannel<T> {
    tx: mpsc::UnboundedSender<T>,
    rx: mpsc::UnboundedReceiver<T>,
}

impl<T> ExchangeChannel<T> {
    /// Construct a new [`Self`].
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }
}

impl<T> Default for ExchangeChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}
