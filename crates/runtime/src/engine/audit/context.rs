use crate::Sequence;
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

/// `Engine` context that an [`AuditTick`](super::AuditTick) was generated in.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct EngineContext {
    pub sequence: Sequence,
    pub time: DateTime<Utc>,
}
