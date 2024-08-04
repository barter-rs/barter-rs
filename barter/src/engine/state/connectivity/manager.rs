use crate::engine::state::{connectivity::ConnectivityState, EngineState};
use barter_instrument::exchange::{ExchangeId, ExchangeIndex};

pub trait ConnectivityManager<ExchangeKey> {
    fn connectivity(&self, key: &ExchangeKey) -> &ConnectivityState;
    fn connectivity_mut(&mut self, key: &ExchangeKey) -> &mut ConnectivityState;
}

impl<Market, Strategy, Risk, AssetKey, InstrumentKey> ConnectivityManager<ExchangeIndex>
    for EngineState<Market, Strategy, Risk, ExchangeIndex, AssetKey, InstrumentKey>
{
    fn connectivity(&self, key: &ExchangeIndex) -> &ConnectivityState {
        self.connectivity
            .0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    fn connectivity_mut(&mut self, key: &ExchangeIndex) -> &mut ConnectivityState {
        self.connectivity
            .0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey> ConnectivityManager<ExchangeId>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
{
    fn connectivity(&self, key: &ExchangeId) -> &ConnectivityState {
        self.connectivity
            .0
            .get(key)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    fn connectivity_mut(&mut self, key: &ExchangeId) -> &mut ConnectivityState {
        self.connectivity
            .0
            .get_mut(key)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }
}
