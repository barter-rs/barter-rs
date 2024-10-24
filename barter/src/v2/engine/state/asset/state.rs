use barter_instrument::asset::Asset;
use crate::v2::balance::{AssetBalance, Balance};
use crate::v2::Snapshot;

#[derive(Debug)]
pub struct AssetState {
    pub asset: Asset,
    pub balance: Balance,
}

impl AssetState {
    pub fn update_from_balance<AssetKey>(&mut self, balance: Snapshot<&AssetBalance<AssetKey>>) {
        let Snapshot(balance) = balance;
        self.balance = balance.balance;
    }
}