use super::{StreamBuilder, Streams};
use crate::{
    error::DataError,
    streams::{consumer::MarketStreamResult, reconnect::stream::ReconnectingStream},
    subscription::SubscriptionKind,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::channel::Channel;
use futures_util::StreamExt;
use std::{collections::HashMap, fmt::Debug, future::Future, pin::Pin};

/// Communicative type alias representing the [`Future`] result of a [`StreamBuilder::init`] call
/// generated whilst executing [`MultiStreamBuilder::add`].
pub type BuilderInitFuture = Pin<Box<dyn Future<Output = Result<(), DataError>>>>;

/// Builder to configure and initialise a common [`Streams<Output>`](Streams) instance from
/// multiple [`StreamBuilder<SubscriptionKind>`](StreamBuilder)s.
#[derive(Default)]
pub struct MultiStreamBuilder<Output> {
    pub channels: HashMap<ExchangeId, Channel<Output>>,
    pub futures: Vec<BuilderInitFuture>,
}

impl<Output> Debug for MultiStreamBuilder<Output>
where
    Output: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiStreamBuilder<Output>")
            .field("channels", &self.channels)
            .field("num_futures", &self.futures.len())
            .finish()
    }
}

impl<Output> MultiStreamBuilder<Output> {
    /// Construct a new [`Self`].
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            futures: Vec::new(),
        }
    }

    /// Add a [`StreamBuilder<SubscriptionKind>`](StreamBuilder) to the [`MultiStreamBuilder`]. Creates a
    /// [`Future`] that calls [`StreamBuilder::init`] and maps the [`SubscriptionKind::Event`](SubscriptionKind)
    /// into a common `Output`.
    ///
    /// Note that the created [`Future`] is not awaited until the [`MultiStreamBuilder::init`]
    /// method is invoked.
    #[allow(clippy::should_implement_trait)]
    pub fn add<InstrumentKey, Kind>(mut self, builder: StreamBuilder<InstrumentKey, Kind>) -> Self
    where
        Output:
            From<MarketStreamResult<InstrumentKey, Kind::Event>> + Debug + Clone + Send + 'static,
        InstrumentKey: Debug + Send + 'static,
        Kind: SubscriptionKind + 'static,
        Kind::Event: Send,
    {
        // Allocate HashMap to hold the exchange_tx<Output> for each StreamBuilder exchange present
        let mut exchange_txs = HashMap::with_capacity(builder.channels.len());

        // Iterate over each StreamBuilder exchange present
        for exchange in builder.channels.keys().cloned() {
            // Insert ExchangeChannel<Output> Entry to Self for each exchange
            let exchange_tx = self.channels.entry(exchange).or_default().tx.clone();

            // Insert new exchange_tx<Output> into HashMap for each exchange
            exchange_txs.insert(exchange, exchange_tx);
        }

        // Init Streams<Kind::Event> & send mapped Outputs to the associated exchange_tx
        self.futures.push(Box::pin(async move {
            builder
                .init()
                .await?
                .streams
                .into_iter()
                .for_each(|(exchange, exchange_rx)| {
                    // Remove exchange_tx<Output> from HashMap that's associated with this tuple:
                    // (ExchangeId, exchange_rx<MarketStreamResult<InstrumentKey, SubscriptionKind::Event>>)
                    let exchange_tx = exchange_txs
                        .remove(&exchange)
                        .expect("all exchange_txs should be present here");

                    // Task to receive MarketStreamResult<SubscriptionKind::Event> and send Outputs via exchange_tx
                    tokio::spawn(
                        exchange_rx
                            .into_stream()
                            .map(Output::from)
                            .forward_to(exchange_tx),
                    );
                });

            Ok(())
        }));

        self
    }

    /// Initialise each [`StreamBuilder<SubscriptionKind>`](StreamBuilder) that was added to the
    /// [`MultiStreamBuilder`] and map all [`Streams<SubscriptionKind::Event>`](Streams) into a common
    /// [`Streams<Output>`](Streams).
    pub async fn init(self) -> Result<Streams<Output>, DataError> {
        // Await Stream initialisation perpetual and ensure success
        futures::future::try_join_all(self.futures).await?;

        // Construct Streams<Output> using each ExchangeChannel receiver
        Ok(Streams {
            streams: self
                .channels
                .into_iter()
                .map(|(exchange, channel)| (exchange, channel.rx))
                .collect(),
        })
    }
}
