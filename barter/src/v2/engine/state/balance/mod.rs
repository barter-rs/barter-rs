use crate::v2::{
    balance::{AssetBalance, Balance},
    engine::state::UpdateFromKeyedSnapshot,
    Snapshot,
};
use barter_instrument::exchange::ExchangeId;
use derive_more::{Constructor, From};
use fnv::FnvHashMap;
use itertools::Either;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, hash::Hash};
use tracing::warn;
use vecmap::VecMap;

pub trait BalanceManager<AssetKey>
where
    Self: UpdateFromKeyedSnapshot<Vec<AssetBalance<AssetKey>>, Key = ExchangeId>,
{
    fn update_from_balance(
        &mut self,
        exchange: &ExchangeId,
        snapshot: Snapshot<&AssetBalance<AssetKey>>,
    );

    fn balance(&self, exchange: &ExchangeId, asset: &AssetKey) -> Option<&Balance>;

    fn balances_by_exchange<'a>(
        &'a self,
        exchange: &ExchangeId,
    ) -> impl Iterator<Item = (&'a AssetKey, &'a Balance)>
    where
        AssetKey: 'a;
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From, Constructor)]
pub struct Balances<AssetKey: Eq + Hash>(pub VecMap<ExchangeId, FnvHashMap<AssetKey, Balance>>);

impl<AssetKey> UpdateFromKeyedSnapshot<Vec<AssetBalance<AssetKey>>> for Balances<AssetKey>
where
    AssetKey: Debug + Clone + Eq + Hash,
{
    type Key = ExchangeId;

    fn update_from_keyed_snapshot(
        &mut self,
        key: &Self::Key,
        snapshot: &Vec<AssetBalance<AssetKey>>,
    ) {
        let Some(exchange_state) = self.0.get_mut(key) else {
            warn!(
                exchange = %key,
                event = ?snapshot,
                "BalanceManager ignoring keyed snapshot received for non-configured exchange"
            );
            return;
        };

        *exchange_state = snapshot
            .iter()
            .map(|balance| (balance.asset.clone(), balance.balance))
            .collect()
    }
}

impl<AssetKey> BalanceManager<AssetKey> for Balances<AssetKey>
where
    AssetKey: Debug + Clone + Eq + Hash,
{
    fn update_from_balance(
        &mut self,
        exchange: &ExchangeId,
        snapshot: Snapshot<&AssetBalance<AssetKey>>,
    ) {
        let Some(exchange_balances) = self.0.get_mut(exchange) else {
            warn!(
                %exchange,
                asset = ?snapshot.0.asset,
                event = ?snapshot,
                "BalanceManager ignoring Snapshot<AssetBalance> received for non-configured exchange",
            );
            return;
        };

        let Snapshot(AssetBalance { asset, balance }) = snapshot;

        let Some(asset_balance) = exchange_balances.get_mut(asset) else {
            warn!(
                %exchange,
                asset = ?snapshot.0.asset,
                event = ?snapshot,
                "BalanceManager ignoring Snapshot<AssetBalance> received for non-configured asset",
            );
            return;
        };

        *asset_balance = *balance;
    }

    fn balance(&self, exchange: &ExchangeId, asset: &AssetKey) -> Option<&Balance> {
        self.0
            .get(exchange)
            .and_then(|balances| balances.get(asset))
    }

    fn balances_by_exchange<'a>(
        &'a self,
        exchange: &ExchangeId,
    ) -> impl Iterator<Item = (&'a AssetKey, &'a Balance)>
    where
        AssetKey: 'a,
    {
        self.0.get(exchange).map_or_else(
            || {
                warn!(
                    %exchange,
                    "BalanceManager cannot return balances for non-configured exchange",
                );
                Either::Left(std::iter::empty())
            },
            |balances| Either::Right(balances.iter()),
        )
    }
}

impl<AssetKey> Default for Balances<AssetKey>
where
    AssetKey: Eq + Hash,
{
    fn default() -> Self {
        Self(VecMap::default())
    }
}
