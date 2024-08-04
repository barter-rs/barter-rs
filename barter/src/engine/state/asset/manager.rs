use crate::engine::state::{
    asset::{AssetState, AssetStates},
    EngineState,
};
use barter_instrument::asset::{name::AssetNameInternal, AssetIndex, ExchangeAsset};

pub trait AssetStateManager<AssetKey> {
    type State;

    fn asset(&self, key: &AssetKey) -> &Self::State;
    fn asset_mut(&mut self, key: &AssetKey) -> &mut Self::State;
}

impl<Market, Strategy, Risk, ExchangeKey, InstrumentKey> AssetStateManager<AssetIndex>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetIndex, InstrumentKey>
{
    type State = AssetState;

    fn asset(&self, key: &AssetIndex) -> &Self::State {
        self.assets.asset(key)
    }

    fn asset_mut(&mut self, key: &AssetIndex) -> &mut Self::State {
        self.assets.asset_mut(key)
    }
}

impl<Market, Strategy, Risk, ExchangeKey, InstrumentKey>
    AssetStateManager<ExchangeAsset<AssetNameInternal>>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetNameInternal, InstrumentKey>
{
    type State = AssetState;

    fn asset(&self, key: &ExchangeAsset<AssetNameInternal>) -> &Self::State {
        self.assets.asset(key)
    }

    fn asset_mut(&mut self, key: &ExchangeAsset<AssetNameInternal>) -> &mut Self::State {
        self.assets.asset_mut(key)
    }
}

impl AssetStateManager<AssetIndex> for AssetStates {
    type State = AssetState;

    fn asset(&self, key: &AssetIndex) -> &Self::State {
        self.0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key}"))
    }

    fn asset_mut(&mut self, key: &AssetIndex) -> &mut Self::State {
        self.0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key}"))
    }
}

impl AssetStateManager<ExchangeAsset<AssetNameInternal>> for AssetStates {
    type State = AssetState;

    fn asset(&self, key: &ExchangeAsset<AssetNameInternal>) -> &Self::State {
        self.0
            .get(key)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key:?}"))
    }

    fn asset_mut(&mut self, key: &ExchangeAsset<AssetNameInternal>) -> &mut Self::State {
        self.0
            .get_mut(key)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key:?}"))
    }
}
