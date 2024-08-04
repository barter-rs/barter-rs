use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration of an [`InstrumentKind::Future`](super::InstrumentKind) contract.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
pub struct FutureContract {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub expiry: DateTime<Utc>,
}
