use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct AssetBalance<AssetKey> {
    pub asset: AssetKey,
    pub balance: Balance,
}

#[derive(
    Debug, Copy, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize, Constructor,
)]
pub struct Balance {
    pub total: Decimal,
    pub free: Decimal,
}

impl Balance {
    pub fn used(&self) -> Decimal {
        self.total - self.free
    }
}
