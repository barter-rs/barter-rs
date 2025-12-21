/// `NoneOneOrMany` enum.
pub mod none_one_or_many;

/// `OneOrMany` enum.
pub mod one_or_many;

/// `Snapshot<T>` new type wrapper.
pub mod snapshot;

pub type FnvIndexMap<K, V> = indexmap::IndexMap<K, V, fnv::FnvBuildHasher>;
pub type FnvIndexSet<T> = indexmap::IndexSet<T, fnv::FnvBuildHasher>;
