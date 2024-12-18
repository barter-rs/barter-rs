use crate::FnvIndexMap;
use barter_execution::balance::{AssetBalance, Balance};
use barter_instrument::{
    asset::{name::AssetNameInternal, Asset, ExchangeAsset},
    index::IndexedInstruments,
};
use barter_integration::snapshot::Snapshot;
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod manager;

/// Collection of exchange [`AssetState`]s indexed by [`AssetIndex`].
///
/// Note that the same named assets on different exchanges will have their own [`AssetState`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetStates(pub FnvIndexMap<ExchangeAsset<AssetNameInternal>, AssetState>);

/// Represents the current state of an asset, including its balance and last update
/// `time_exchange`.
///
/// When used in the context of [`AssetStates`], this state is for an exchange asset, but it could
/// be used for a "global" asset that aggregates data for similar named assets on multiple
/// exchanges.
///
/// # Fields
/// * `asset` - Asset information including internal and exchange names
/// * `balance` - Current balance information containing total and free amounts
/// * `time_exchange` - Timestamp of the last state update from the exchange
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct AssetState {
    /// `Asset` name data that details the internal and exchange names.
    pub asset: Asset,

    /// Current balance of the asset.
    pub balance: Balance,

    /// Exchange timestamp of the last snapshot.
    pub time_exchange: DateTime<Utc>,
}

impl AssetState {
    /// Updates the `AssetState` from an [`AssetBalance`] snapshot, if the snapshot is more recent.
    ///
    /// This method ensures temporal consistency by only applying updates from snapshots that
    /// are at least as recent as the current state.
    pub fn update_from_balance<AssetKey>(&mut self, snapshot: Snapshot<&AssetBalance<AssetKey>>) {
        let Snapshot(snapshot) = snapshot;
        if self.time_exchange <= snapshot.time_exchange {
            self.time_exchange = snapshot.time_exchange;
            self.balance = snapshot.balance;
        }
    }
}

/// Generates an indexed [`AssetStates`] containing default asset balance data.
///
/// Note that the `time_exchange` is set to `DateTime::<Utc>::MIN_UTC`
///
/// # Arguments
/// * `instruments` - Reference to [`IndexedInstruments`] containing asset specification data.
pub fn generate_empty_asset_states(instruments: &IndexedInstruments) -> AssetStates {
    AssetStates(
        instruments
            .assets()
            .iter()
            .map(|asset| {
                (
                    ExchangeAsset::new(
                        asset.value.exchange,
                        asset.value.asset.name_internal.clone(),
                    ),
                    AssetState::new(
                        asset.value.asset.clone(),
                        Balance::default(),
                        DateTime::<Utc>::MIN_UTC,
                    ),
                )
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::asset_state;
    use barter_instrument::asset::name::AssetNameExchange;
    use chrono::TimeZone;

    #[test]
    fn test_update_from_balance_with_more_recent_snapshot() {
        let mut state = asset_state("btc", 1000.0, 900.0, DateTime::<Utc>::MIN_UTC);

        let snapshot = Snapshot(AssetBalance {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: 1000.0,
                free: 800.0,
            },
            time_exchange: DateTime::<Utc>::MAX_UTC,
        });

        state.update_from_balance(snapshot.as_ref());

        let expected = asset_state("btc", 1000.0, 800.0, DateTime::<Utc>::MAX_UTC);

        assert_eq!(state, expected)
    }

    #[test]
    fn test_update_from_balance_with_equal_timestamp() {
        // Test case: Verify state updates when snapshot has equal timestamp
        let time = Utc.timestamp_opt(1000, 0).unwrap();

        let mut state = asset_state("btc", 1000.0, 900.0, time);

        let snapshot = Snapshot(AssetBalance {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: 1000.0,
                free: 800.0,
            },
            time_exchange: time,
        });

        state.update_from_balance(snapshot.as_ref());

        assert_eq!(state.balance.total, 1000.0);
        assert_eq!(state.balance.free, 800.0);
        assert_eq!(state.time_exchange, time);
    }

    #[test]
    fn test_update_from_balance_with_stale_snapshot() {
        let mut state = AssetState {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: 1000.0,
                free: 900.0,
            },
            time_exchange: DateTime::<Utc>::MAX_UTC,
        };

        let snapshot = Snapshot(AssetBalance {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: 1000.0,
                free: 800.0,
            },
            time_exchange: DateTime::<Utc>::MIN_UTC,
        });

        state.update_from_balance(snapshot.as_ref());

        let expected = AssetState {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: 1000.0,
                free: 900.0,
            },
            time_exchange: DateTime::<Utc>::MAX_UTC,
        };

        assert_eq!(state, expected)
    }
}
