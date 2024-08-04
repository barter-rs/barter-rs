use barter_instrument::{
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

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

impl ConnectivityStates {
    /// Updates from an exchange AccountStream disconnection.
    ///
    /// Sets the account `ConnectivityState` for the provided `ExchangeId`
    /// to [`Health::Reconnecting`].
    pub fn update_from_account_reconnecting(&mut self, exchange: &ExchangeId) {
        warn!(%exchange, "EngineState received AccountStream disconnected event");
        self.global = Health::Reconnecting;
        self.connectivity_mut(exchange).account = Health::Reconnecting;
    }

    /// Updates from an exchange AccountStream event, setting the `ConnectivityState` account
    /// connection to [`Health::Healthy`] if it was not previously.
    ///
    /// If after the update all `ConnectivityState`s are healthy, the global health is set to
    /// `Health::Healthy`.
    pub fn update_from_account_event(&mut self, exchange: &ExchangeIndex) {
        if self.global == Health::Healthy {
            return;
        }

        let state = self.connectivity_index_mut(exchange);
        if state.account == Health::Healthy {
            return;
        }

        info!(
            %exchange,
            "EngineState received AccountStream event - setting connection to Healthy"
        );
        state.account = Health::Healthy;

        if self.exchange_states().all(ConnectivityState::all_healthy) {
            info!("EngineState setting global connectivity to Healthy");
            self.global = Health::Healthy
        }
    }

    /// Updates from an exchange MarketStream disconnection.
    ///
    /// Sets the market data `ConnectivityState` for the provided `ExchangeId`
    /// to [`Health::Reconnecting`].
    pub fn update_from_market_reconnecting(&mut self, exchange: &ExchangeId) {
        warn!(%exchange, "EngineState received MarketStream disconnect event");
        self.global = Health::Reconnecting;
        self.connectivity_mut(exchange).market_data = Health::Reconnecting
    }

    /// Updates from an exchange MarketStream event, setting the `ConnectivityState` market data
    /// connection to [`Health::Healthy`] if it was not previously.
    ///
    /// If after the update all `ConnectivityState`s are healthy, the global health is set to
    /// `Health::Healthy`.
    pub fn update_from_market_event(&mut self, exchange: &ExchangeId) {
        if self.global == Health::Healthy {
            return;
        }

        let state = self.connectivity_mut(exchange);
        if state.market_data == Health::Healthy {
            return;
        }

        info!(
            %exchange,
            "EngineState received MarketStream event - setting connection to Healthy"
        );
        state.market_data = Health::Healthy;

        if self.exchange_states().all(ConnectivityState::all_healthy) {
            info!("EngineState setting global connectivity to Healthy");
            self.global = Health::Healthy
        }
    }

    /// Returns a reference to the `ConnectivityState` associated with the
    /// provided `ExchangeIndex`.
    ///
    /// Panics if the `ConnectivityState` associated with the `ExchangeIndex` is not found.
    pub fn connectivity_index(&self, key: &ExchangeIndex) -> &ConnectivityState {
        self.exchanges
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    /// Returns a mutable reference to the `ConnectivityState` associated with the
    /// provided `ExchangeIndex`.
    ///
    /// Panics if the `ConnectivityState` associated with the `ExchangeIndex` is not found.
    pub fn connectivity_index_mut(&mut self, key: &ExchangeIndex) -> &mut ConnectivityState {
        self.exchanges
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    /// Returns a reference to the `ConnectivityState` associated with the
    /// provided `ExchangeId`.
    ///
    /// Panics if the `ConnectivityState` associated with the `ExchangeId` is not found.
    pub fn connectivity(&self, key: &ExchangeId) -> &ConnectivityState {
        self.exchanges
            .get(key)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    /// Returns a mutable reference to the `ConnectivityState` associated with the
    /// provided `ExchangeId`.
    ///
    /// Panics if the `ConnectivityState` associated with the `ExchangeId` is not found.
    pub fn connectivity_mut(&mut self, key: &ExchangeId) -> &mut ConnectivityState {
        self.exchanges
            .get_mut(key)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    /// Return an `Iterator` of the `ExchangeId`s being tracked.
    pub fn exchange_ids(&self) -> impl Iterator<Item = &ExchangeId> {
        self.exchanges.keys()
    }

    /// Return an `Iterator` of all `ConnectivityState`s being tracked.
    pub fn exchange_states(&self) -> impl Iterator<Item = &ConnectivityState> {
        self.exchanges.values()
    }
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
