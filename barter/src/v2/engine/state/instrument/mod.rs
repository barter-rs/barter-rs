use crate::v2::{
    engine::state::{instrument::manager::InstrumentFilter, order::Orders},
    execution::InstrumentAccountSnapshot,
    order::{Open, Order},
    position::Position,
    trade::Trade,
    Snapshot,
};
use barter_instrument::instrument::{name::InstrumentNameInternal, Instrument};
use derive_more::Constructor;
use indexmap::IndexMap;
use itertools::Either;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod manager;
pub mod market_data;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InstrumentStates<Market, ExchangeKey, AssetKey, InstrumentKey>(
    pub  IndexMap<
        InstrumentNameInternal,
        InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>,
    >,
);

impl<Market, ExchangeKey, AssetKey, InstrumentKey>
    InstrumentStates<Market, ExchangeKey, AssetKey, InstrumentKey>
{
    pub fn states<'a>(
        &'a self,
        filter: &'a InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> impl Iterator<Item = &'a InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>>
    where
        Market: 'a,
        ExchangeKey: PartialEq + 'a,
        AssetKey: 'a,
        InstrumentKey: PartialEq + 'a,
    {
        match filter {
            InstrumentFilter::None => Either::Left(Either::Left(self.0.values())),
            InstrumentFilter::Exchanges(exchanges) => Either::Left(Either::Right(
                self.0
                    .values()
                    .filter(|state| exchanges.contains(&state.instrument.exchange)),
            )),
            InstrumentFilter::Instruments(instruments) => Either::Right(
                self.0
                    .values()
                    .filter(|state| instruments.contains(&state.position.instrument)),
            ),
            InstrumentFilter::Underlyings(_underlying) => {
                todo!()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey> {
    pub instrument: Instrument<ExchangeKey, AssetKey>,
    pub position: Position<InstrumentKey>,
    pub orders: Orders<ExchangeKey, InstrumentKey>,
    pub market: Market,
}

impl<Market, ExchangeKey, AssetKey, InstrumentKey>
    InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>
where
    ExchangeKey: Clone,
    InstrumentKey: Clone,
{
    pub fn update_from_account_snapshot(
        &mut self,
        snapshot: &InstrumentAccountSnapshot<ExchangeKey, InstrumentKey>,
    ) {
        self.update_from_position_snapshot(Snapshot(&snapshot.position));
        self.update_from_opens_snapshot(Snapshot(&snapshot.orders))
    }

    pub fn update_from_position_snapshot(&mut self, position: Snapshot<&Position<InstrumentKey>>) {
        let _ = std::mem::replace(&mut self.position, position.0.clone());
    }

    pub fn update_from_trade(&mut self, _trade: &Trade<AssetKey, InstrumentKey>) {
        todo!()
    }

    pub fn update_from_opens_snapshot(
        &mut self,
        orders: Snapshot<&Vec<Order<ExchangeKey, InstrumentKey, Open>>>,
    ) {
        let _ = std::mem::replace(
            &mut self.orders.0,
            orders
                .0
                .iter()
                .map(|order| (order.cid, Order::from(order.clone())))
                .collect(),
        );
    }
}
