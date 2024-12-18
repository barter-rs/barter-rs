use barter_instrument::{exchange::ExchangeId, index::IndexedInstruments};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub mod manager;

#[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct ConnectivityStates(pub IndexMap<ExchangeId, ConnectivityState>);

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize)]
pub struct ConnectivityState {
    pub market_data: Connection,
    pub account: Connection,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum Connection {
    Healthy,
    Reconnecting,
}

impl Default for Connection {
    fn default() -> Self {
        Self::Healthy
    }
}

pub fn generate_default_connectivity_states(
    instruments: &IndexedInstruments,
) -> ConnectivityStates {
    ConnectivityStates(
        instruments
            .exchanges()
            .iter()
            .map(|exchange| (exchange.value, ConnectivityState::default()))
            .collect(),
    )
}
