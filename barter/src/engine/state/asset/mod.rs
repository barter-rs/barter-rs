pub mod manager;

use barter_execution::balance::{AssetBalance, Balance};
use barter_instrument::{
    asset::{name::AssetNameInternal, Asset, ExchangeAsset},
    index::IndexedInstruments,
};
use barter_integration::snapshot::Snapshot;
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetStates(pub IndexMap<ExchangeAsset<AssetNameInternal>, AssetState>);

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct AssetState {
    pub asset: Asset,
    pub balance: Balance,
    pub time_exchange: DateTime<Utc>,
}

impl AssetState {
    pub fn update_from_balance<AssetKey>(&mut self, snapshot: Snapshot<&AssetBalance<AssetKey>>) {
        let Snapshot(snapshot) = snapshot;
        if self.time_exchange <= snapshot.time_exchange {
            self.time_exchange = snapshot.time_exchange;
            self.balance = snapshot.balance;
        }
    }
}

pub fn generate_default_asset_states(instruments: &IndexedInstruments) -> AssetStates {
    AssetStates(
        instruments
            .assets
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
    use barter_instrument::asset::name::AssetNameExchange;

    #[test]
    fn test_update_from_balance_with_more_recent_snapshot() {
        let mut state = AssetState {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: 1000.0,
                free: 900.0,
            },
            time_exchange: Utc::now(),
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
            time_exchange: DateTime::<Utc>::MAX_UTC,
        });

        state.update_from_balance(snapshot.as_ref());

        let expected = AssetState {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: 1000.0,
                free: 800.0,
            },
            time_exchange: DateTime::<Utc>::MAX_UTC,
        };

        assert_eq!(state, expected)
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
