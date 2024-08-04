pub mod none_one_or_many;
pub mod one_or_many;

pub type FnvIndexMap<K, V> = indexmap::IndexMap<K, V, fnv::FnvBuildHasher>;
pub type FnvIndexSet<T> = indexmap::IndexSet<T, fnv::FnvBuildHasher>;
