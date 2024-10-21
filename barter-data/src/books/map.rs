use crate::books::OrderBook;
use derive_more::Constructor;
use fnv::FnvHashMap;
use parking_lot::RwLock;
use std::{hash::Hash, sync::Arc};

/// Collection of shared-state Instrument [`OrderBook`]s. Manage the local books using
/// the [`super::manager`] module, and then clone the map for viewing the up to
/// date [`OrderBook`]s elsewhere.
///
/// See [`OrderBookMapSingle`] and [`OrderBookMapMulti`] for implementations.
pub trait OrderBookMap: Clone {
    type Key;

    /// Return an [`Iterator`] over the [`OrderBookMap`] Keys (eg/ InstrumentKey).
    fn keys(&self) -> impl Iterator<Item = &Self::Key>;

    /// Attempt to find the [`OrderBook`] associated with the provided Key.
    fn find(&self, key: &Self::Key) -> Option<Arc<RwLock<OrderBook>>>;
}

/// Single Instrument [`OrderBook`] wrapped in a shared-state lock.
#[derive(Debug, Clone, Constructor)]
pub struct OrderBookMapSingle<Key> {
    pub instrument: Key,
    pub book: Arc<RwLock<OrderBook>>,
}

impl<Key> OrderBookMap for OrderBookMapSingle<Key>
where
    Key: PartialEq + Clone,
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

/// Multiple Instrument [`OrderBook`] wrapped in a shared-state lock.
#[derive(Debug, Clone, Constructor)]
pub struct OrderBookMapMulti<Key>
where
    Key: Eq + Hash,
{
    pub books: FnvHashMap<Key, Arc<RwLock<OrderBook>>>,
}

impl<Key> OrderBookMap for OrderBookMapMulti<Key>
where
    Key: Clone + Eq + Hash,
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
    /// Insert a new [`OrderBook`] into the [`OrderBookMapMulti`].
    pub fn insert(&mut self, instrument: Key, book: Arc<RwLock<OrderBook>>) {
        self.books.insert(instrument, book);
    }
}
