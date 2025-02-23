use super::Streams;
use crate::{
    Identifier,
    error::DataError,
    exchange::StreamSelector,
    instrument::InstrumentData,
    streams::{
        consumer::{MarketStreamResult, STREAM_RECONNECTION_POLICY, init_market_stream},
        reconnect::stream::ReconnectingStream,
    },
    subscription::{Subscription, SubscriptionKind},
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{Validator, channel::Channel};
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    future::Future,
    pin::Pin,
};

/// Defines the [`MultiStreamBuilder`](multi::MultiStreamBuilder) API for ergonomically
/// initialising a common [`Streams<Output>`](Streams) from multiple
/// [`StreamBuilder<SubscriptionKind>`](StreamBuilder)s.
pub mod multi;

/// Defines the [`DynamicStreams`](dynamic::DynamicStreams) API for initialising an arbitrary number
/// of `MarketStream`s from the [`ExchangeId`] and [`SubKind`](crate::subscription::SubKind) enums, rather than concrete
/// types.
pub mod dynamic;

/// Communicative type alias representing the [`Future`] result of a [`Subscription`] validation
/// call generated whilst executing [`StreamBuilder::subscribe`].
pub type SubscribeFuture = Pin<Box<dyn Future<Output = Result<(), DataError>>>>;

/// Builder to configure and initialise a [`Streams<MarketEvent<SubscriptionKind::Event>`](Streams) instance
/// for a specific [`SubscriptionKind`].
#[derive(Default)]
pub struct StreamBuilder<InstrumentKey, Kind>
where
    Kind: SubscriptionKind,
{
    pub channels: HashMap<ExchangeId, Channel<MarketStreamResult<InstrumentKey, Kind::Event>>>,
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
        Instrument: InstrumentData<Key = InstrumentKey> + Ord + Display + 'static,
        Instrument::Key: Debug + Clone + Send + 'static,
        Kind: Ord + Display + Send + Sync + 'static,
        Kind::Event: Clone + Send,
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
            let stream = init_market_stream(STREAM_RECONNECTION_POLICY, subscriptions).await?;

            // Forward MarketEvents to ExchangeTx
            tokio::spawn(stream.forward_to(exchange_tx));

            Ok(())
        }));

        self
    }

    /// Spawn a [`MarketStreamResult<SubscriptionKind::Event>`](MarketStreamResult) consumer loop
    /// for each collection of [`Subscription`]s added to [`StreamBuilder`] via the
    /// [`subscribe()`](StreamBuilder::subscribe()) method.
    ///
    /// Each consumer loop distributes consumed [`MarketStreamResult`] to
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
