use crate::books::OrderBook;
use barter_instrument::instrument::InstrumentIndex;
use barter_integration::collection::FnvIndexMap;
use derive_more::Constructor;
use fnv::FnvHashMap;
use parking_lot::RwLock;
use std::fmt::Debug;
use std::{fmt::Display, hash::Hash, sync::Arc};

/// Collection of shared-state Instrument [`OrderBook`]s. Manage the local books using
/// the [`super::manager`] module, and then clone the map for viewing the up to
/// date [`OrderBook`]s elsewhere.
///
/// See [`OrderBookMapSingle`] and [`OrderBookMapMulti`] for implementations.
pub trait OrderBookMap: Clone {
    type StoredKey;
    type LookupKey;

    /// Return an [`Iterator`] over the [`OrderBookMap`] Keys (eg/ InstrumentKey).
    fn keys(&self) -> impl Iterator<Item = &Self::StoredKey>;

    /// Attempt to find the [`OrderBook`] associated with the provided Key.
    fn find(&self, key: &Self::LookupKey) -> Option<Arc<RwLock<OrderBook>>>;
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
    type StoredKey = Key;
    type LookupKey = Key;

    fn keys(&self) -> impl Iterator<Item = &Self::StoredKey> {
        std::iter::once(&self.instrument)
    }

    fn find(&self, key: &Self::LookupKey) -> Option<Arc<RwLock<OrderBook>>> {
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
    type StoredKey = Key;
    type LookupKey = Key;

    fn keys(&self) -> impl Iterator<Item = &Self::StoredKey> {
        self.books.keys()
    }

    fn find(&self, key: &Self::LookupKey) -> Option<Arc<RwLock<OrderBook>>> {
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

#[derive(Debug, Clone, Constructor)]

pub struct IndexedOrderBookMapMulti<Key>(pub FnvIndexMap<Key, Arc<RwLock<OrderBook>>>);

impl<Key> OrderBookMap for IndexedOrderBookMapMulti<Key>
where
    Key: Clone + Eq + Hash + Debug,
    InstrumentIndex: Debug,
{
    type StoredKey = Key;
    type LookupKey = InstrumentIndex;

    fn keys(&self) -> impl Iterator<Item = &Self::StoredKey> {
        self.0.keys()
    }

    fn find(&self, key: &Self::LookupKey) -> Option<Arc<RwLock<OrderBook>>> {
        // self.0.get(key).cloned()
        self.0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .cloned()
    }
}

impl<Key> IndexedOrderBookMapMulti<Key>
where
    Key: Clone + Eq + Hash + Display,
{
    /// Returns a shared reference to the `Arc<RwLock<OrderBook>>` associated with the given `StoredKey`.
    ///
    /// This allows accessing the `OrderBook` using the `StoredKey` directly, without needing the original [`InstrumentIndex`].
    ///
    /// # Panics
    /// Panics if no `OrderBook` is associated with the provided `StoredKey`.
    pub fn instrument(&self, key: &Key) -> &Arc<RwLock<OrderBook>> {
        self.0
            .get(key)
            .unwrap_or_else(|| panic!("IndexedOrderBookMapMulti does not contain: {key}"))
    }
}
