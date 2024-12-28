use crate::{statistic::summary::asset::TearSheetAssetGenerator, FnvIndexMap, Timed};
use barter_execution::balance::{AssetBalance, Balance};
use barter_instrument::{
    asset::{name::AssetNameInternal, Asset, AssetIndex, ExchangeAsset},
    index::IndexedInstruments,
};
use barter_integration::snapshot::Snapshot;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Collection of exchange [`AssetState`]s indexed by [`AssetIndex`].
///
/// Note that the same named assets on different exchanges will have their own [`AssetState`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetStates(pub FnvIndexMap<ExchangeAsset<AssetNameInternal>, AssetState>);

impl AssetStates {
    /// Return a reference to the `AssetState` associated with an `AssetIndex`.
    ///
    /// Panics if `AssetState` does not exist.
    pub fn asset_index(&self, key: &AssetIndex) -> &AssetState {
        self.0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key}"))
    }

    /// Return a mutable reference to the `AssetState` associated with an `AssetIndex`.
    ///
    /// Panics if `AssetState` does not exist.
    pub fn asset_index_mut(&mut self, key: &AssetIndex) -> &mut AssetState {
        self.0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key}"))
    }

    /// Return a reference to the `AssetState` associated with an `ExchangeAsset<AssetNameInternal>`.
    ///
    /// Panics if `AssetState` does not exist.
    pub fn asset(&self, key: &ExchangeAsset<AssetNameInternal>) -> &AssetState {
        self.0
            .get(key)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key:?}"))
    }

    /// Return a mutable reference to the `AssetState` associated with an
    /// `ExchangeAsset<AssetNameInternal>`.
    ///
    /// Panics if `AssetState` does not exist.
    pub fn asset_mut(&mut self, key: &ExchangeAsset<AssetNameInternal>) -> &mut AssetState {
        self.0
            .get_mut(key)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key:?}"))
    }
}

/// Represents the current state of an asset, including its [`Balance`] and last update
/// `time_exchange`.
///
/// When used in the context of [`AssetStates`], this state is for an exchange asset, but it could
/// be used for a "global" asset that aggregates data for similar named assets on multiple
/// exchanges.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct AssetState {
    /// `Asset` name data that details the internal and exchange names.
    pub asset: Asset,

    /// TearSheet generator for summarising trading session changes the asset.
    pub statistics: TearSheetAssetGenerator,

    /// Current balance of the asset and associated exchange timestamp.
    pub balance: Option<Timed<Balance>>,
}

impl AssetState {
    /// Updates the `AssetState` from an [`AssetBalance`] snapshot, if the snapshot is more recent.
    ///
    /// This method ensures temporal consistency by only applying updates from snapshots that
    /// are at least as recent as the current state.
    pub fn update_from_balance<AssetKey>(&mut self, snapshot: Snapshot<&AssetBalance<AssetKey>>) {
        let Some(balance) = &mut self.balance else {
            self.balance = Some(Timed::new(snapshot.0.balance, snapshot.0.time_exchange));
            self.statistics.update_from_balance(snapshot);
            return;
        };

        if balance.time <= snapshot.value().time_exchange {
            balance.time = snapshot.value().time_exchange;
            balance.value = snapshot.value().balance;
            self.statistics.update_from_balance(snapshot);
        }
    }
}

/// Generates an indexed [`AssetStates`] containing default asset balance data.
///
/// Note that the `time_exchange` is set to `DateTime::<Utc>::MIN_UTC`
pub fn generate_empty_indexed_asset_states(instruments: &IndexedInstruments) -> AssetStates {
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
                        TearSheetAssetGenerator::default(),
                        None,
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
    use chrono::{DateTime, TimeZone, Utc};
    use rust_decimal_macros::dec;

    #[test]
    fn test_update_from_balance_with_first_ever_snapshot() {
        let mut state = AssetState {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("btc"),
            },
            statistics: Default::default(),
            balance: None,
        };

        let snapshot = Snapshot(AssetBalance {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("btc"),
            },
            balance: Balance {
                total: dec!(1100.0),
                free: dec!(1100.0),
            },
            time_exchange: DateTime::<Utc>::MIN_UTC,
        });

        state.update_from_balance(snapshot.as_ref());

        let expected = asset_state("btc", 1100.0, 1100.0, DateTime::<Utc>::MIN_UTC);

        assert_eq!(state, expected)
    }

    #[test]
    fn test_update_from_balance_with_more_recent_snapshot() {
        let mut state = asset_state("btc", 1000.0, 1000.0, DateTime::<Utc>::MIN_UTC);

        let snapshot = Snapshot(AssetBalance {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: dec!(1100.0),
                free: dec!(1100.0),
            },
            time_exchange: DateTime::<Utc>::MAX_UTC,
        });

        state.update_from_balance(snapshot.as_ref());

        let expected = asset_state("btc", 1100.0, 1100.0, DateTime::<Utc>::MAX_UTC);

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
                total: dec!(1000.0),
                free: dec!(800.0),
            },
            time_exchange: time,
        });

        state.update_from_balance(snapshot.as_ref());

        assert_eq!(state.balance.unwrap().value.total, dec!(1000.0));
        assert_eq!(state.balance.unwrap().value.free, dec!(800.0));
        assert_eq!(state.balance.unwrap().time, time);
    }

    #[test]
    fn test_update_from_balance_with_stale_snapshot() {
        let mut state = asset_state("btc", 1000.0, 900.0, DateTime::<Utc>::MAX_UTC);

        let snapshot = Snapshot(AssetBalance {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: dec!(1000.0),
                free: dec!(800.0),
            },
            time_exchange: DateTime::<Utc>::MIN_UTC,
        });

        state.update_from_balance(snapshot.as_ref());

        let expected = asset_state("btc", 1000.0, 900.0, DateTime::<Utc>::MAX_UTC);

        assert_eq!(state, expected)
    }
}
