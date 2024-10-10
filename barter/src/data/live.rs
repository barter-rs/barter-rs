use super::{Feed, MarketGenerator};
use barter_data::event::{DataKind, MarketEvent};
use barter_data::streams::consumer::MarketStreamEvent;
use barter_data::streams::reconnect;
use barter_integration::model::instrument::Instrument;
use futures::executor::{block_on_stream, BlockingStream};
use futures::Stream;
use tokio::sync::mpsc;

/// Live [`Feed`] of market events.
#[derive(Debug)]
pub struct MarketFeed<Event> {
    pub market_rx: mpsc::UnboundedReceiver<Event>,
}

impl<Event> MarketGenerator<Event> for MarketFeed<Event> {
    fn next(&mut self) -> Feed<Event> {
        loop {
            match self.market_rx.try_recv() {
                Ok(event) => break Feed::Next(event),
                Err(mpsc::error::TryRecvError::Empty) => continue,
                Err(mpsc::error::TryRecvError::Disconnected) => break Feed::Finished,
            }
        }
    }
}

impl<Event> MarketFeed<Event> {
    /// Initialises a live [`MarketFeed`] that yields market `Event`s from the provided
    /// [`mpsc::UnboundedReceiver`].
    ///
    /// Recommended use with the `Barter-Data` [`Streams`](barter_data::streams::Streams):
    ///  1. Initialise a [`Streams`](barter_data::streams::Streams) using the
    ///     [`StreamBuilder`](barter_data::streams::builder::StreamBuilder) or
    ///     [`MultiStreamBuilder`](barter_data::streams::builder::multi::MultiStreamBuilder).
    ///  2. Use [`Streams::join`](barter_data::streams::Streams::join) to join all exchange
    ///     [`mpsc::UnboundedReceiver`] streams into a unified [`mpsc::UnboundedReceiver`].
    ///  3. Construct [`Self`] with the unified [`mpsc::UnboundedReceiver`].
    pub fn new(market_rx: mpsc::UnboundedReceiver<Event>) -> Self {
        Self { market_rx }
    }
}

/// Live [`Feed`] of market events.
#[derive(Debug)]
pub struct ReconnectingMarketFeed<St: Stream + Unpin> {
    pub market_stream: BlockingStream<St>,
}

impl<St> MarketGenerator<MarketEvent> for ReconnectingMarketFeed<St>
where
    St: Stream<Item = MarketStreamEvent<Instrument, DataKind>> + Unpin,
{
    fn next(&mut self) -> Feed<MarketEvent> {
        self.market_stream
            .next()
            .map(|event| match event {
                reconnect::Event::Reconnecting(_) => Feed::Unhealthy,
                reconnect::Event::Item(item) => Feed::Next(item),
            })
            .unwrap_or(Feed::Finished)
    }
}

impl<St> ReconnectingMarketFeed<St>
where
    St: Stream + Unpin,
{
    /// Initialises a live [`ReconnectingMarketFeed`] that yields market events from the
    /// provided `Stream`.
    ///
    /// Recommended use with the `Barter-Data` [`Streams`](barter_data::streams::Streams):
    ///  1. Initialise a [`Streams`](barter_data::streams::Streams) using the
    ///     [`StreamBuilder`](barter_data::streams::builder::StreamBuilder) or
    ///     [`MultiStreamBuilder`](barter_data::streams::builder::multi::MultiStreamBuilder).
    ///  2. Use [`Streams::select_all`](barter_data::streams::Streams::select_all) to select and
    ///     merge every exchange `Stream` using  all futures_util::stream::select_all.
    ///  3. Construct [`Self`] with the merged `Stream`.
    pub fn new(market_stream: St) -> Self {
        Self {
            market_stream: block_on_stream(market_stream),
        }
    }
}
