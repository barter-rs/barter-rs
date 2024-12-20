use crate::engine::state::{
    connectivity::{ConnectivityState, Health},
    EngineState,
};
use barter_instrument::exchange::{ExchangeId, ExchangeIndex};

pub trait ConnectivityManager<ExchangeKey> {
    fn global_health(&self) -> Health;
    fn global_health_mut(&mut self) -> &mut Health;
    fn connectivity(&self, key: &ExchangeKey) -> &ConnectivityState;
    fn connectivity_mut(&mut self, key: &ExchangeKey) -> &mut ConnectivityState;
    fn connectivities(&self) -> impl Iterator<Item = &ConnectivityState>;
}

impl<Market, Strategy, Risk, AssetKey, InstrumentKey> ConnectivityManager<ExchangeIndex>
    for EngineState<Market, Strategy, Risk, ExchangeIndex, AssetKey, InstrumentKey>
{
    fn global_health(&self) -> Health {
        self.connectivity.global
    }

    fn global_health_mut(&mut self) -> &mut Health {
        &mut self.connectivity.global
    }

    fn connectivity(&self, key: &ExchangeIndex) -> &ConnectivityState {
        self.connectivity
            .exchanges
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    fn connectivity_mut(&mut self, key: &ExchangeIndex) -> &mut ConnectivityState {
        self.connectivity
            .exchanges
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    fn connectivities(&self) -> impl Iterator<Item = &ConnectivityState> {
        self.connectivity.exchanges.values()
    }
}

impl<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey> ConnectivityManager<ExchangeId>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
{
    fn global_health(&self) -> Health {
        self.connectivity.global
    }

    fn global_health_mut(&mut self) -> &mut Health {
        &mut self.connectivity.global
    }

    fn connectivity(&self, key: &ExchangeId) -> &ConnectivityState {
        self.connectivity
            .exchanges
            .get(key)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    fn connectivity_mut(&mut self, key: &ExchangeId) -> &mut ConnectivityState {
        self.connectivity
            .exchanges
            .get_mut(key)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    fn connectivities(&self) -> impl Iterator<Item = &ConnectivityState> {
        self.connectivity.exchanges.values()
    }
}
