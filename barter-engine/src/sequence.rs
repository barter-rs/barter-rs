use derive_more::{Constructor, Display};
use serde::{Deserialize, Serialize};

/// Value and it's associated [`Sequence`].
pub struct Sequenced<T> {
    pub value: T,
    pub sequence: Sequence,
}

/// Monotonically increasing event sequence. Used to track `Engine` event processing sequence.
#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Display,
    Deserialize,
    Serialize,
    Constructor,
)]
pub struct Sequence(pub u64);

impl Sequence {
    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn fetch_add(&mut self) -> Sequence {
        let sequence = *self;
        self.0 += 1;
        sequence
    }
}
