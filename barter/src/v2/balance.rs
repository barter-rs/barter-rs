use crate::v2::instrument::asset::AssetId;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct AssetBalance<Key = AssetId> {
    pub asset: Key,
    pub balance: Balance,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct Balance {
    pub total: f64,
    pub free: f64,
}

impl Balance {
    pub fn used(&self) -> f64 {
        self.total - self.free
    }
}
