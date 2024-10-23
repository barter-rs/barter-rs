use crate::v2::{
    balance::{AssetBalance, Balance},
    Snapshot,
};
use barter_instrument::asset::{Asset, AssetId, AssetIndex, ExchangeAssetKey};
use indexmap::IndexMap;

pub struct AssetStates(pub IndexMap<ExchangeAssetKey<AssetId>, AssetState>);

impl AssetStates {
    pub fn state_by_index(&self, asset: AssetIndex) -> &AssetState {
        self.0
            .get_index(asset.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetIndex: {asset} not present in assets"))
    }

    pub fn state_by_index_mut(&mut self, asset: AssetIndex) -> &mut AssetState {
        self.0
            .get_index_mut(asset.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetIndex: {asset} not present in assets"))
    }

    pub fn update_from_balance(&mut self, balance: Snapshot<&AssetBalance<AssetIndex>>) {
        let Snapshot(balance) = balance;
        self.state_by_index_mut(balance.asset).balance = balance.balance
    }

    pub fn update_from_balances(&mut self, balances: Snapshot<&Vec<AssetBalance<AssetIndex>>>) {
        let Snapshot(balances) = balances;
        for balance in balances {
            self.update_from_balance(Snapshot(balance));
        }
    }
}

pub struct AssetState {
    pub asset: Asset,
    pub balance: Balance,
}
