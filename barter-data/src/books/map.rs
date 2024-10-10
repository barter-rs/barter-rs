use fnv::FnvHashMap;
use parking_lot::RwLock;
use std::hash::Hash;
use std::sync::Arc;
use crate::books::OrderBook;

/// Todo: this is what a user would put in EngineState, where the book is managed elsewhere by
///       an OrderBook manager
pub trait OrderBookMap {
    type Key;

    fn keys(&self) -> impl Iterator<Item = &Self::Key>;
    fn find(&self, key: &Self::Key) -> Option<Arc<RwLock<OrderBook>>>;
}

#[derive(Debug, Clone)]
pub struct OrderBookMapSingle<Key> {
    pub instrument: Key,
    pub book: Arc<RwLock<OrderBook>>,
}

impl<Key> OrderBookMap for OrderBookMapSingle<Key>
where
    Key: PartialEq,
{
    type Key = Key;

    fn keys(&self) -> impl Iterator<Item = &Self::Key> {
        std::iter::once(&self.instrument)
    }

    fn find(&self, key: &Self::Key) -> Option<Arc<RwLock<OrderBook>>> {
        if &self.instrument == key {
            Some(self.book.clone())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderBookMapMulti<Key>
where
    Key: Eq + Hash,
{
    pub books: FnvHashMap<Key, Arc<RwLock<OrderBook>>>,
}

impl<Key> OrderBookMap for OrderBookMapMulti<Key>
where
    Key: Eq + Hash,
{
    type Key = Key;

    fn keys(&self) -> impl Iterator<Item = &Self::Key> {
        self.books.keys()
    }

    fn find(&self, key: &Self::Key) -> Option<Arc<RwLock<OrderBook>>> {
        self.books.get(key).cloned()
    }
}

impl<Key> OrderBookMapMulti<Key>
where
    Key: Eq + Hash,
{
    pub fn insert(&mut self, instrument: Key, book: Arc<RwLock<OrderBook>>) {
        self.books.insert(instrument, book);
    }
}
