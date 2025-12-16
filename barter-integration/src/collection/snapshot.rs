use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Deserialize,
    Serialize,
    Constructor,
    From,
)]
pub struct Snapshot<T>(pub T);

impl<T> Snapshot<T> {
    pub fn value(&self) -> &T {
        &self.0
    }

    pub fn as_ref(&self) -> Snapshot<&T> {
        let Self(item) = self;
        Snapshot(item)
    }

    pub fn map<F, N>(self, op: F) -> Snapshot<N>
    where
        F: Fn(T) -> N,
    {
        let Self(item) = self;
        Snapshot(op(item))
    }
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct SnapUpdates<Snapshot, Updates> {
    pub snapshot: Snapshot,
    pub updates: Updates,
}
