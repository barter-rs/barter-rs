use crate::{
    UnindexedAccountSnapshot,
    balance::AssetBalance,
    order::{
        Order,
        id::ClientOrderId,
        state::{Cancelled, Open},
    },
    trade::Trade,
};
use barter_instrument::{
    asset::{QuoteAsset, name::AssetNameExchange},
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use fnv::FnvHashMap;

#[derive(Debug, Constructor)]
pub struct AccountState {
    balances: FnvHashMap<AssetNameExchange, AssetBalance<AssetNameExchange>>,
    orders_open: FnvHashMap<ClientOrderId, Order<ExchangeId, InstrumentNameExchange, Open>>,
    orders_cancelled:
        FnvHashMap<ClientOrderId, Order<ExchangeId, InstrumentNameExchange, Cancelled>>,
    trades: Vec<Trade<QuoteAsset, InstrumentNameExchange>>,
}

impl AccountState {
    pub fn update_time_exchange(&mut self, time_exchange: DateTime<Utc>) {
        for balance in self.balances.values_mut() {
            balance.time_exchange = time_exchange;
        }

        for order in self.orders_open.values_mut() {
            order.state.time_exchange = time_exchange;
        }
    }

    pub fn balances(&self) -> impl Iterator<Item = &AssetBalance<AssetNameExchange>> + '_ {
        self.balances.values()
    }

    pub fn orders_open(
        &self,
    ) -> impl Iterator<Item = &Order<ExchangeId, InstrumentNameExchange, Open>> + '_ {
        self.orders_open.values()
    }

    pub fn orders_cancelled(
        &self,
    ) -> impl Iterator<Item = &Order<ExchangeId, InstrumentNameExchange, Cancelled>> + '_ {
        self.orders_cancelled.values()
    }

    pub fn trades(
        &self,
        time_since: DateTime<Utc>,
    ) -> impl Iterator<Item = &Trade<QuoteAsset, InstrumentNameExchange>> + '_ {
        self.trades
            .iter()
            .filter(move |trade| trade.time_exchange >= time_since)
    }

    pub fn balance_mut(
        &mut self,
        asset: &AssetNameExchange,
    ) -> Option<&mut AssetBalance<AssetNameExchange>> {
        self.balances.get_mut(asset)
    }

    pub fn ack_trade(&mut self, trade: Trade<QuoteAsset, InstrumentNameExchange>) {
        self.trades.push(trade);
    }
}

impl From<UnindexedAccountSnapshot> for AccountState {
    fn from(value: UnindexedAccountSnapshot) -> Self {
        let UnindexedAccountSnapshot { exchange: _, balances } = value;

        let balances = balances
            .into_iter()
            .map(|asset_balance| (asset_balance.asset.clone(), asset_balance))
            .collect();

        Self {
            balances,
            orders_open: FnvHashMap::default(),
            orders_cancelled: FnvHashMap::default(),
            trades: vec![],
        }
    }
}
