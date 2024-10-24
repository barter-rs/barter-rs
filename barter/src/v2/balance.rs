use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use barter_instrument::exchange::ExchangeId;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct ExchangeAssetBalance<AssetKey> {
    pub exchange: ExchangeId,
    pub asset: AssetKey,
    pub balance: Balance,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct AssetBalance<AssetKey> {
    pub asset: AssetKey,
    pub balance: Balance,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct Balance {
    pub total: Decimal,
    pub free: Decimal,
}

impl Balance {
    pub fn used(&self) -> Decimal {
        self.total - self.free
    }
}
