use super::{Feed, MarketGenerator};
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
