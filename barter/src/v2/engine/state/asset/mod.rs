pub mod manager;

use crate::v2::{
    balance::{AssetBalance, Balance},
    Snapshot,
};
use barter_instrument::asset::{name::AssetNameInternal, Asset, ExchangeAsset};
use derive_more::Constructor;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetStates(pub IndexMap<ExchangeAsset<AssetNameInternal>, AssetState>);

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
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
