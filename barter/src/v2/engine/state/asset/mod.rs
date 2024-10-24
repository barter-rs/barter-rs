use std::hash::Hash;
use barter_instrument::asset::{AssetIndex, ExchangeAsset};
use indexmap::IndexMap;
use crate::v2::engine::state::asset::state::AssetState;

pub mod state;

#[derive(Debug)]
pub struct AssetStates<AssetKey>(pub IndexMap<ExchangeAsset<AssetKey>, AssetState>);

impl<AssetKey> AssetStates<AssetKey> {
    pub fn state(&self, asset: &ExchangeAsset<AssetKey>) -> Option<&AssetState>
    where
        AssetKey: Eq + Hash,
    {
        self.0.get(asset)
    }

    pub fn state_mut(&mut self, asset: &ExchangeAsset<AssetKey>) -> Option<&mut AssetState>
    where
        AssetKey: Eq + Hash,
    {
        self.0.get_mut(asset)
    }

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
}

