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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_value() {
        let snap = Snapshot::new(42);
        assert_eq!(snap.value(), &42);
    }

    #[test]
    fn test_snapshot_as_ref() {
        let snap = Snapshot::new(String::from("hello"));
        let snap_ref = snap.as_ref();
        assert_eq!(*snap_ref.value(), "hello");
    }

    #[test]
    fn test_snapshot_map() {
        let snap = Snapshot::new(10);
        let doubled = snap.map(|x| x * 2);
        assert_eq!(doubled.value(), &20);
    }

    #[test]
    fn test_snapshot_serde_round_trip() {
        let snap = Snapshot::new(42u64);
        let json = serde_json::to_string(&snap).unwrap();
        let deserialised: Snapshot<u64> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialised, snap);
    }

    #[test]
    fn test_snap_updates_construction() {
        let snap_updates = SnapUpdates::new(Snapshot::new(1), vec![2, 3, 4]);
        assert_eq!(snap_updates.snapshot, Snapshot::new(1));
        assert_eq!(snap_updates.updates, vec![2, 3, 4]);
    }

    #[test]
    fn test_snap_updates_serde_round_trip() {
        let snap_updates = SnapUpdates::new(Snapshot::new("state"), vec!["a", "b"]);
        let json = serde_json::to_string(&snap_updates).unwrap();
        let deserialised: SnapUpdates<Snapshot<&str>, Vec<&str>> =
            serde_json::from_str(&json).unwrap();
        assert_eq!(deserialised, snap_updates);
    }
}
