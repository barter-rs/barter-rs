use chrono::{DateTime, Utc};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct AssetBalance<AssetKey> {
    pub asset: AssetKey,
    pub balance: Balance,
    pub time_exchange: DateTime<Utc>,
}

#[derive(
    Debug, Copy, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize, Constructor,
)]
pub struct Balance {
    pub total: f64,
    pub free: f64,
}

impl Balance {
    pub fn used(&self) -> f64 {
        self.total - self.free
    }
}
