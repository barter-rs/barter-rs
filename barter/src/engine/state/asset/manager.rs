use crate::engine::state::{asset::AssetState, EngineState};
use barter_instrument::asset::{name::AssetNameInternal, AssetIndex, ExchangeAsset};

pub trait AssetStateManager<AssetKey> {
    fn asset(&self, key: &AssetKey) -> &AssetState;
    fn asset_mut(&mut self, key: &AssetKey) -> &mut AssetState;
}

impl<Market, Strategy, Risk, ExchangeKey, InstrumentKey> AssetStateManager<AssetIndex>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetIndex, InstrumentKey>
{
    fn asset(&self, key: &AssetIndex) -> &AssetState {
        self.assets
            .0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key}"))
    }

    fn asset_mut(&mut self, key: &AssetIndex) -> &mut AssetState {
        self.assets
            .0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk, ExchangeKey, InstrumentKey>
    AssetStateManager<ExchangeAsset<AssetNameInternal>>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetNameInternal, InstrumentKey>
{
    fn asset(&self, key: &ExchangeAsset<AssetNameInternal>) -> &AssetState {
        self.assets
            .0
            .get(key)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key:?}"))
    }

    fn asset_mut(&mut self, key: &ExchangeAsset<AssetNameInternal>) -> &mut AssetState {
        self.assets
            .0
            .get_mut(key)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key:?}"))
    }
}
