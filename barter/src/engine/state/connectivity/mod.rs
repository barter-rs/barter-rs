use barter_instrument::{exchange::ExchangeId, index::IndexedInstruments};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub mod manager;

/// Collection of exchange [`ConnectivityState`]s indexed by [`ExchangeId`].
///
/// Maintains the connection status of market data and account connections for each exchange.
#[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct ConnectivityStates(pub IndexMap<ExchangeId, ConnectivityState>);

/// Represents the current connection state for both market data and account connections of an
/// exchange.
///
/// Connection health is monitored separately for market data and account connections since they
/// often use different endpoints and may have different health states.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize)]
pub struct ConnectivityState {
    /// Status of market data connection.
    pub market_data: Connection,

    /// Status of the account and execution connection.
    pub account: Connection,
}

/// Represents the health status of a connection to an exchange endpoint.
///
/// Used to track both market data and account connections in a [`ConnectivityState`].
///
/// Default implementation is [`Connection::Healthy`].
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum Connection {
    /// Connection is established and functioning normally.
    Healthy,

    /// Connection is currently attempting to re-establish after a disconnect or failure.
    Reconnecting,
}

impl Default for Connection {
    fn default() -> Self {
        Self::Healthy
    }
}

/// Generates an indexed [`ConnectivityStates`] containing default connection states.
///
/// Creates a new connection state tracker for each exchange in the provided instruments, with all
/// connections initially set to [`Connection::Healthy`].
///
/// # Arguments
/// * `instruments` - Reference to [`IndexedInstruments`] containing what exchanges are being tracked.
pub fn generate_empty_indexed_connectivity_states(
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
