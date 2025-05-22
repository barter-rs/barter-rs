use crate::error::JackbotError;
use jackbot_data::streams::consumer::MarketStreamEvent;
use jackbot_instrument::instrument::InstrumentIndex;
use chrono::{DateTime, Utc};
use futures::Stream;
use std::{future::Future, sync::Arc};

/// Interface that provides the backtest MarketStream and associated [`HistoricalClock`].
pub trait BacktestMarketData {
    /// The type of market events provided by this data source.
    type Kind;

    /// Return the `DateTime<Utc>` of the first event in the market data `Stream`.
    fn time_first_event(&self) -> impl Future<Output = Result<DateTime<Utc>, JackbotError>>;

    /// Return a `Stream` of `MarketStreamEvent`s.
    fn stream(
        &self,
    ) -> impl Future<
        Output = Result<
            impl Stream<Item = MarketStreamEvent<InstrumentIndex, Self::Kind>> + Send + 'static,
            JackbotError,
        >,
    >;
}

/// In-memory market data.
///
/// Stores all market events in memory and generates a `Stream` of [`MarketStreamEvent`] by
/// lazy cloning the data as it's required.
#[derive(Debug, Clone)]
pub struct MarketDataInMemory<Kind> {
    time_first_event: DateTime<Utc>,
    events: Arc<Vec<MarketStreamEvent<InstrumentIndex, Kind>>>,
}

impl<Kind> BacktestMarketData for MarketDataInMemory<Kind>
where
    Kind: Clone + Sync + Send + 'static,
{
    type Kind = Kind;

    async fn time_first_event(&self) -> Result<DateTime<Utc>, JackbotError> {
        Ok(self.time_first_event)
    }

    async fn stream(
        &self,
    ) -> Result<
        impl Stream<Item = MarketStreamEvent<InstrumentIndex, Self::Kind>> + Send + 'static,
        JackbotError,
    > {
        let events = Arc::clone(&self.events);
        let lazy_clone_iter = (0..events.len()).map(move |index| events[index].clone());
        let stream = futures::stream::iter(lazy_clone_iter);
        Ok(stream)
    }
}

impl<Kind> MarketDataInMemory<Kind> {
    /// Create a new in-memory market data source from a vector of market events.
    pub fn new(events: Arc<Vec<MarketStreamEvent<InstrumentIndex, Kind>>>) -> Self {
        let time_first_event = events
            .iter()
            .find_map(|event| match event {
                MarketStreamEvent::Item(event) => Some(event.time_exchange),
                _ => None,
            })
            .expect("cannot construct MarketDataInMemory using an empty Vec<MarketStreamEvent>");

        Self {
            time_first_event,
            events,
        }
    }

    /// Create a new in-memory market data source using a [`DataLoader`].
    pub async fn from_loader<L>(loader: L) -> Result<Self, JackbotError>
    where
        L: crate::backtest::data_loader::DataLoader<Kind = Kind> + Send,
        Kind: Clone + Sync + Send + 'static,
    {
        let events = loader.load().await?;
        Ok(Self::new(Arc::new(events)))
    }
}
