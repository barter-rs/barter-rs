use crate::v2::{
    balance::{AssetBalance, Balance},
    instrument::asset::AssetId,
    Snapshot,
};
use barter_integration::model::Exchange;
use derive_more::{Constructor, From};
use itertools::Either;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::warn;
use vecmap::VecMap;

pub trait BalanceManager<AssetKey> {
    fn balance(&self, exchange: &Exchange, asset: &AssetKey) -> Option<&Balance>;

    fn balances_by_exchange<'a>(
        &'a self,
        exchange: &Exchange,
    ) -> impl Iterator<Item = (&'a AssetKey, &'a Balance)>
    where
        AssetKey: 'a;

    fn update_from_snapshot(
        &mut self,
        exchange: &Exchange,
        snapshot: Snapshot<&AssetBalance<AssetKey>>,
    );
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize, From, Constructor)]
pub struct Balances(pub VecMap<Exchange, VecMap<AssetId, Balance>>);

impl BalanceManager<AssetId> for Balances {
    fn balance(&self, exchange: &Exchange, asset: &AssetId) -> Option<&Balance> {
        self.0
            .get(exchange)
            .and_then(|balances| balances.get(asset))
    }

    fn balances_by_exchange<'a>(
        &'a self,
        exchange: &Exchange,
    ) -> impl Iterator<Item = (&'a AssetId, &'a Balance)>
    where
        AssetId: 'a,
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

    fn update_from_snapshot(
        &mut self,
        exchange: &Exchange,
        snapshot: Snapshot<&AssetBalance<AssetId>>,
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
}
