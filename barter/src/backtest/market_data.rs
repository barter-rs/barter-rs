use crate::{engine::clock::HistoricalClock, error::BarterError};
use barter_data::streams::consumer::MarketStreamEvent;
use barter_instrument::instrument::InstrumentIndex;
use futures::Stream;
use std::sync::Arc;

/// Interface that provides the backtest MarketStream and associated [`HistoricalClock`].
pub trait BacktestMarketData {
    /// The type of market events provided by this data source.
    type Kind;

    /// Generate a `HistoricalClock` and `Stream` of `MarketStreamEvent`s.
    ///
    /// The returned `HistoricalClock` should be initialised to the start of the historical data
    /// period of the market data.
    fn generate(
        &self,
    ) -> impl Future<
        Output = Result<
            (
                HistoricalClock,
                impl Stream<Item = MarketStreamEvent<InstrumentIndex, Self::Kind>> + Send + 'static,
            ),
            BarterError,
        >,
    >;
}

/// In-memory market data.
///
/// Stores all market events in memory and generates a `Stream` of [`MarketStreamEvent`] by
/// lazy cloning the data as it's required.
#[derive(Debug, Clone)]
pub struct MarketDataInMemory<Kind> {
    /// Clock initialized to the timestamp of the first market event.
    clock: HistoricalClock,
    /// Vector of all market events to replay during the backtest.
    events: Arc<Vec<MarketStreamEvent<InstrumentIndex, Kind>>>,
}

impl<Kind> BacktestMarketData for MarketDataInMemory<Kind>
where
    Kind: Clone + Sync + Send + 'static,
{
    type Kind = Kind;

    async fn generate(
        &self,
    ) -> Result<
        (
            HistoricalClock,
            impl Stream<Item = MarketStreamEvent<InstrumentIndex, Self::Kind>> + Send + 'static,
        ),
        BarterError,
    > {
        let events = Arc::clone(&self.events);
        let lazy_clone_iter = (0..events.len()).map(move |index| events[index].clone());
        let stream = futures::stream::iter(lazy_clone_iter);

        Ok((self.clock.clone(), stream))
    }
}

impl<Kind> MarketDataInMemory<Kind> {
    /// Create a new in-memory market data source from a vector of market events.
    ///
    /// Initialises a [`HistoricalClock`] using the `time_exchange` of the first market event.
    pub fn new(events: Arc<Vec<MarketStreamEvent<InstrumentIndex, Kind>>>) -> Self {
        let time_exchange_first = events
            .iter()
            .find_map(|event| match event {
                MarketStreamEvent::Item(event) => Some(event.time_exchange),
                _ => None,
            })
            .expect("cannot construct MarketDataInMemory using an empty Vec<MarketStreamEvent>");

        Self {
            clock: HistoricalClock::new(time_exchange_first),
            events,
        }
    }
}
