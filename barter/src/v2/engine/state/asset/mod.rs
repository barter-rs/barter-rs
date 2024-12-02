pub mod manager;

use crate::v2::{
    balance::{AssetBalance, Balance},
    Snapshot,
};
use barter_instrument::asset::{name::AssetNameInternal, Asset, ExchangeAsset};
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
}

impl AssetState {
    pub fn update_from_balance<AssetKey>(&mut self, balance: Snapshot<&AssetBalance<AssetKey>>) {
        let Snapshot(balance) = balance;
        self.balance = balance.balance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_instrument::asset::name::AssetNameExchange;
    use rust_decimal_macros::dec;

    #[test]
    fn test_update_from_balance() {
        let mut state = AssetState {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: dec!(1000.0),
                free: dec!(900.0),
            },
        };

        let snapshot = Snapshot(AssetBalance {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: dec!(1000.0),
                free: dec!(800.0),
            },
        });

        state.update_from_balance(snapshot.as_ref());

        let expected = AssetState {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance {
                total: dec!(1000.0),
                free: dec!(800.0),
            },
        };

        assert_eq!(state, expected)
    }
}
