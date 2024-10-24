use std::hash::Hash;
use crate::v2::{
    engine::state::order_manager::{OrderManager},
    execution::{error::ExecutionError, InstrumentAccountSnapshot},
    order::{
        Cancelled, ExchangeOrderState, InternalOrderState, Open, Order, RequestCancel, RequestOpen,
    },
    Snapshot,
};
use barter_instrument::{
    instrument::{InstrumentIndex},
};
use indexmap::IndexMap;
use barter_instrument::instrument::InstrumentId;
use crate::v2::engine::state::instrument::state::InstrumentState;
use crate::v2::engine::state::StateManager;

pub mod state;
pub mod market_data;


#[derive(Debug)]
pub struct InstrumentStates<AssetKey, InstrumentKey, Market>(
    pub IndexMap<InstrumentKey, InstrumentState<AssetKey, InstrumentKey, Market>>
);

// impl<AssetKey, InstrumentKey, Market> InstrumentStates<AssetKey, InstrumentKey, Market> {
//     pub fn state(&self, instrument: &InstrumentKey) -> Option<&InstrumentState<AssetKey, InstrumentKey, Market>>
//     where
//         InstrumentKey: Eq + Hash,
//     {
//         self.0.get(instrument)
//     }
// 
//     pub fn state_mut(&mut self, instrument: &InstrumentKey) -> Option<&mut InstrumentState<AssetKey, InstrumentKey, Market>>
//     where
//         InstrumentKey: Eq + Hash,
//     {
//         self.0.get_mut(instrument)
//     }
// 
//     pub fn state_by_index(&self, instrument: InstrumentIndex) -> &InstrumentState<AssetKey, InstrumentKey, Market> {
//         self.0
//             .get_index(instrument.index())
//             .map(|(_key, state)| state)
//             .unwrap_or_else(|| panic!("InstrumentIndex: {instrument} not present in instruments"))
//     }
// 
//     pub fn state_by_index_mut(
//         &mut self,
//         instrument: InstrumentIndex,
//     ) -> &mut InstrumentState<AssetKey, InstrumentKey, Market> {
//         self.0
//             .get_index_mut(instrument.index())
//             .map(|(_key, state)| state)
//             .unwrap_or_else(|| panic!("InstrumentIndex: {instrument} not present in instruments"))
//     }
// }

impl<AssetKey, InstrumentKey, Market> InstrumentStates<AssetKey, InstrumentKey, Market>
where
    InstrumentKey: Clone,
{
    pub fn update_from_account_snapshots(
        &mut self,
        snapshots: &[InstrumentAccountSnapshot<InstrumentIndex>],
    ) {
        for snapshot in snapshots {
            // Find InstrumentState associated with snapshot
            let state = self
                .state_mut(&snapshot.position.instrument)
                .expect("urgh todo: ");

            // Replace Instrument Position
            state.update_from_position_snapshot(Snapshot(&snapshot.position));

            // Replace Instrument Orders (wipes all open & cancel in-flight requests)
            state.update_from_opens_snapshot(Snapshot(&snapshot.orders));
        }
    }
}



impl<AssetKey, Market> OrderManager<InstrumentIndex> for InstrumentStates<AssetKey, InstrumentIndex, Market> {
    fn orders<'a>(&'a self) -> impl Iterator<Item = &'a Order<InstrumentIndex, InternalOrderState>>
    where
        InstrumentIndex: 'a,
    {
        self.0.values().flat_map(|state| state.orders.orders())
    }

    fn record_in_flight_cancel(&mut self, request: &Order<InstrumentIndex, RequestCancel>) {
        self.state_by_index_mut(request.instrument)
            .orders
            .record_in_flight_cancel(request)
    }

    fn record_in_flight_open(&mut self, request: &Order<InstrumentIndex, RequestOpen>) {
        self.state_by_index_mut(request.instrument)
            .orders
            .record_in_flight_open(request)
    }
    fn update_from_open(
        &mut self,
        response: &Order<InstrumentIndex, Result<Open, ExecutionError>>,
    ) {
        self.state_by_index_mut(response.instrument)
            .orders
            .update_from_open(response)
    }

    fn update_from_cancel(
        &mut self,
        response: &Order<InstrumentIndex, Result<Cancelled, ExecutionError>>,
    ) {
        self.state_by_index_mut(response.instrument)
            .orders
            .update_from_cancel(response)
    }

    fn update_from_order_snapshot(
        &mut self,
        snapshot: Snapshot<&Order<InstrumentIndex, ExchangeOrderState>>,
    ) {
        self.state_by_index_mut(snapshot.0.instrument)
            .orders
            .update_from_order_snapshot(snapshot)
    }

    fn update_from_opens_snapshot(
        &mut self,
        snapshot: Snapshot<&Vec<Order<InstrumentIndex, Open>>>,
    ) {
        // Todo: Do I want to fully replace existing orders, or change Open to be ExchangeOrderState
        //       and iterate?
        let Snapshot(_orders) = snapshot;
        todo!()
    }
}

impl<AssetKey, InstrumentKey, Market> StateManager<InstrumentIndex>
for InstrumentStates<AssetKey, InstrumentKey, Market>
{
    type State = InstrumentState<AssetKey, InstrumentKey, Market>;

    fn state(&self, key: &InstrumentIndex) -> Option<&Self::State> {
        self.0
            .get_index(key.index())
            .map(|(_key, state)| state)
    }

    fn state_mut(&mut self, key: &InstrumentIndex) -> Option<&mut Self::State> {
        self.0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
    }
}

impl<AssetKey, Market> StateManager<InstrumentId>
for InstrumentStates<AssetKey, InstrumentId, Market>
{
    type State = InstrumentState<AssetKey, InstrumentId, Market>;

    fn state(&self, key: &InstrumentId) -> Option<&Self::State> {
        self.0.get(key)
    }

    fn state_mut(&mut self, key: &InstrumentId) -> Option<&mut Self::State> {
        self.0.get_mut(key)
    }
}