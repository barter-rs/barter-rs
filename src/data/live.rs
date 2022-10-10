use super::{error::DataError, Feed, MarketGenerator};
use barter_data::{
    builder::Streams,
    model::{subscription::Subscription, MarketEvent},
};
use tokio::sync::mpsc;

/// Live [`Feed`] of [`MarketEvent`]s.
#[derive(Debug)]
pub struct MarketFeed {
    pub market_rx: mpsc::UnboundedReceiver<MarketEvent>,
}

impl MarketGenerator for MarketFeed {
    fn generate(&mut self) -> Feed<MarketEvent> {
        loop {
            match self.market_rx.try_recv() {
                Ok(market) => break Feed::Next(market),
                Err(mpsc::error::TryRecvError::Empty) => continue,
                Err(mpsc::error::TryRecvError::Disconnected) => break Feed::Finished,
            }
        }
    }
}

impl MarketFeed {
    /// Initialises a live [`MarketFeed`] that yields [`MarketEvent`]s for each [`Subscription`]
    /// provided.
    ///
    /// Utilises Barter-Data [`Streams`] to establish and maintain healthy connections with the
    /// relevant exchange servers.
    pub async fn init<SubIter, Sub>(subscriptions: SubIter) -> Result<Self, DataError>
    where
        SubIter: IntoIterator<Item = Sub>,
        Sub: Into<Subscription>,
    {
        let streams = Streams::builder().subscribe(subscriptions).init().await?;

        Ok(Self {
            market_rx: streams.join().await,
        })
    }
}
