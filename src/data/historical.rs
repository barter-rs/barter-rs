use crate::data::{Feed, MarketGenerator};

/// Historical [`Feed`] of market events.
#[derive(Debug)]
pub struct MarketFeed<Iter, Event>
where
    Iter: Iterator<Item = Event>,
{
    pub market_iterator: Iter,
}

impl<Iter, Event> MarketGenerator<Event> for MarketFeed<Iter, Event>
where
    Iter: Iterator<Item = Event>,
{
    fn next(&mut self) -> Feed<Event> {
        self.market_iterator
            .next()
            .map_or(Feed::Finished, Feed::Next)
    }
}

impl<Iter, Event> MarketFeed<Iter, Event>
where
    Iter: Iterator<Item = Event>,
{
    /// Construct a historical [`MarketFeed`] that yields market events from the `IntoIterator`
    /// provided.
    pub fn new<IntoIter>(market_iterator: IntoIter) -> Self
    where
        IntoIter: IntoIterator<Item = Event, IntoIter = Iter>,
    {
        Self {
            market_iterator: market_iterator.into_iter(),
        }
    }
}
