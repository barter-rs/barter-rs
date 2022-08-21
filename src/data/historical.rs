use crate::data::{Feed, MarketGenerator};
use barter_data::model::MarketEvent;

/// Historical [`Feed`] of [`MarketEvent`]s.
#[derive(Debug)]
pub struct MarketFeed<I>
where
    I: Iterator<Item = MarketEvent>,
{
    pub market_iterator: I,
}

impl<I> MarketGenerator for MarketFeed<I>
where
    I: Iterator<Item = MarketEvent>,
{
    fn generate(&mut self) -> Feed<MarketEvent> {
        self.market_iterator
            .next()
            .map_or(Feed::Finished, Feed::Next)
    }
}

impl<I> MarketFeed<I>
where
    I: Iterator<Item = MarketEvent>,
{
    /// Construct a historical [`MarketFeed`] that yields [`MarketEvent`]s from the `Iterator`
    /// provided.
    pub fn new(market_iterator: I) -> Self {
        Self { market_iterator }
    }
}
