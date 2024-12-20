use barter_instrument::{exchange::ExchangeId, index::IndexedInstruments};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub mod manager;

/// Maintains a global connection [`Health`], as well as the connection status of market data
/// and account connections for each exchange.
#[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct ConnectivityStates {
    /// Global connection [`Health`].
    ///
    /// Global health is considered `Healthy` if all exchange market data and account
    /// connections are `Healthy`.
    pub global: Health,

    /// Connectivity `Health` of market data and account connections by exchange.
    pub exchanges: IndexMap<ExchangeId, ConnectivityState>,
}

/// Represents the `Health` status of a component or connection to an exchange endpoint.
///
/// Used to track both market data and account connections in a [`ConnectivityState`].
///
/// Default implementation is [`Health::Reconnecting`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum Health {
    /// Connection is established and functioning normally.
    Healthy,

    /// Connection is currently attempting to re-establish after a disconnect or failure.
    Reconnecting,
}

/// Represents the current connection state for both market data and account connections of an
/// exchange.
///
/// Connection health is monitored separately for market data and account connections since they
/// often use different endpoints and may have different health states.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize)]
pub struct ConnectivityState {
    /// Status of market data connection.
    pub market_data: Health,

    /// Status of the account and execution connection.
    pub account: Health,
}

impl ConnectivityState {
    /// Returns true if both market data and account connections are [`Health::Healthy`].
    pub fn all_healthy(&self) -> bool {
        self.market_data == Health::Healthy && self.account == Health::Healthy
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::Reconnecting
    }
}

/// Generates an indexed [`ConnectivityStates`] containing default connection states.
///
/// Creates a new connection state tracker for each exchange in the provided instruments, with all
/// connections initially set to [`Health::Reconnecting`].
///
/// # Arguments
/// * `instruments` - Reference to [`IndexedInstruments`] containing what exchanges are being tracked.
pub fn generate_empty_indexed_connectivity_states(
    instruments: &IndexedInstruments,
) -> ConnectivityStates {
    ConnectivityStates {
        global: Health::Reconnecting,
        exchanges: instruments
            .exchanges()
            .iter()
            .map(|exchange| (exchange.value, ConnectivityState::default()))
            .collect(),
    }
}
