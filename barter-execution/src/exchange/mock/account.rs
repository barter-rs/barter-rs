use crate::{
    UnindexedAccountSnapshot,
    balance::AssetBalance,
    order::{
        Order,
        id::ClientOrderId,
        state::{ActiveOrderState, Cancelled, FullyFilled, InactiveOrderState, Open, OrderState},
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
    orders_filled:
        FnvHashMap<ClientOrderId, Order<ExchangeId, InstrumentNameExchange, FullyFilled>>,
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

    pub fn orders_filled(
        &self,
    ) -> impl Iterator<Item = &Order<ExchangeId, InstrumentNameExchange, FullyFilled>> + '_ {
        self.orders_filled.values()
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

    pub fn record_filled_order(
        &mut self,
        filled_order: Order<ExchangeId, InstrumentNameExchange, FullyFilled>,
    ) {
        self.orders_filled
            .insert(filled_order.key.cid.clone(), filled_order);
    }
}

impl From<UnindexedAccountSnapshot> for AccountState {
    fn from(value: UnindexedAccountSnapshot) -> Self {
        let UnindexedAccountSnapshot {
            exchange: _,
            balances,
            instruments,
        } = value;

        let balances = balances
            .into_iter()
            .map(|asset_balance| (asset_balance.asset.clone(), asset_balance))
            .collect();

        let (orders_open, orders_cancelled, orders_filled) = instruments.into_iter().fold(
            (
                FnvHashMap::default(),
                FnvHashMap::default(),
                FnvHashMap::default(),
            ),
            |(mut orders_open, mut orders_cancelled, mut orders_filled), snapshot| {
                for order in snapshot.orders {
                    match order.state {
                        OrderState::Active(ActiveOrderState::Open(open)) => {
                            orders_open.insert(
                                order.key.cid.clone(),
                                Order {
                                    key: order.key,
                                    side: order.side,
                                    price: order.price,
                                    quantity: order.quantity,
                                    kind: order.kind,
                                    time_in_force: order.time_in_force,
                                    state: open,
                                },
                            );
                        }
                        OrderState::Inactive(InactiveOrderState::Cancelled(cancelled)) => {
                            orders_cancelled.insert(
                                order.key.cid.clone(),
                                Order {
                                    key: order.key,
                                    side: order.side,
                                    price: order.price,
                                    quantity: order.quantity,
                                    kind: order.kind,
                                    time_in_force: order.time_in_force,
                                    state: cancelled,
                                },
                            );
                        }
                        OrderState::Inactive(InactiveOrderState::FullyFilled(fully_filled)) => {
                            orders_filled.insert(
                                order.key.cid.clone(),
                                Order {
                                    key: order.key,
                                    side: order.side,
                                    price: order.price,
                                    quantity: order.quantity,
                                    kind: order.kind,
                                    time_in_force: order.time_in_force,
                                    state: fully_filled,
                                },
                            );
                        }
                        _ => {}
                    }
                }

                (orders_open, orders_cancelled, orders_filled)
            },
        );

        Self {
            balances,
            orders_open,
            orders_cancelled,
            trades: vec![],
            orders_filled,
        }
    }
}
