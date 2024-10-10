use crate::data::{Feed, MarketGenerator};

/// Historical [`Feed`] of market events.
#[derive(Debug)]
pub struct MarketFeed<Iter>
where
    Iter: Iterator,
{
    pub market_iterator: Iter,
}

impl<Iter> MarketGenerator<Iter::Item> for MarketFeed<Iter>
where
    Iter: Iterator,
{
    fn next(&mut self) -> Feed<Iter::Item> {
        self.market_iterator
            .next()
            .map_or(Feed::Finished, Feed::Next)
    }
}

impl<Iter> MarketFeed<Iter>
where
    Iter: Iterator,
{
    /// Construct a historical [`MarketFeed`] that yields market events from the `IntoIterator`
    /// provided.
    pub fn new<IntoIter>(market_iterator: IntoIter) -> Self
    where
        IntoIter: IntoIterator<IntoIter = Iter>,
    {
        Self {
            market_iterator: market_iterator.into_iter(),
        }
    }
}
