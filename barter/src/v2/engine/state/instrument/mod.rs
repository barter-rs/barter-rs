use crate::v2::engine::state::instrument::order::OrderManager;
use crate::v2::engine::state::UpdateFromSnapshot;
use crate::v2::execution::InstrumentAccountSnapshot;
use crate::v2::order::{ExchangeOrderState, InternalOrderState, RequestCancel, RequestOpen};
use crate::v2::position::Position;
use crate::v2::{
    engine::state::instrument::order::Orders,
    execution::error::ExecutionError,
    instrument::Instrument,
    order::{Cancelled, Open, Order},
    Snapshot,
};
use derive_more::{Constructor, From};
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;
use tracing::warn;

pub mod market_data;
pub mod order;
pub mod position;

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize, From)]
pub struct Instruments<InstrumentKey: Eq + Hash, MarketState>(
    pub FnvHashMap<InstrumentKey, InstrumentState<InstrumentKey, MarketState>>,
);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentState<InstrumentKey, MarketState> {
    pub instrument: Instrument<InstrumentKey>,
    pub market: MarketState,
    pub orders: Orders<InstrumentKey>,
    pub position: Position<InstrumentKey>,
}

impl<InstrumentKey, MarketState> UpdateFromSnapshot<Vec<InstrumentAccountSnapshot<InstrumentKey>>>
    for Instruments<InstrumentKey, MarketState>
where
    InstrumentKey: Debug + Clone + Eq + Hash,
{
    fn update_from_snapshot(&mut self, snapshots: &Vec<InstrumentAccountSnapshot<InstrumentKey>>) {
        for snapshot in snapshots {
            let instrument = &snapshot.position.instrument;
            if let Some(state) = self.state_mut(instrument) {
                let _ = std::mem::replace(&mut state.position, snapshot.position.clone());

                // Note: this wipes all open & cancel in-flight requests
                let _ = std::mem::replace(
                    &mut state.orders.inner,
                    snapshot
                        .orders
                        .iter()
                        .map(|order| (order.cid, Order::from(order.clone())))
                        .collect(),
                );
            } else {
                warn!(
                    ?instrument,
                    event = ?snapshot,
                    "EngineState ignoring InstrumentAccountSnapshot received for non-configured instrument"
                );
            }
        }
    }
}

// impl<InstrumentKey, MarketState> MarketDataManager<InstrumentKey> for Instruments<InstrumentKey, MarketState>
// where
//     Self: UpdateFromKeyedSnapshot<MarketState::Snapshot>,
//     InstrumentKey: Debug + Clone + Eq + Hash,
//     MarketState: Debug + MarketDataManager<InstrumentKey>,
//     MarketState::MarketDataKind: Debug,
// {
//     type Snapshot = MarketState::Snapshot;
//     type MarketDataKind = MarketState::MarketDataKind;
//
//     fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey, MarketState::MarketDataKind>) {
//         let Some(state) = self.state_mut(&event.instrument) else {
//             warn!(
//                 exchange = %event.exchange,
//                 instrument = ?event.instrument,
//                 ?event,
//                 "MarketDataManager ignoring MarketEvent received for non-configured instrument",
//             );
//             return;
//         };
//
//         state.market.update_from_market(event);
//     }
// }

impl<InstrumentKey, MarketState> UpdateFromSnapshot<Vec<Order<InstrumentKey, Open>>>
    for Instruments<InstrumentKey, MarketState>
where
    InstrumentKey: Eq + Hash,
{
    fn update_from_snapshot(&mut self, snapshot: &Vec<Order<InstrumentKey, Open>>) {
        todo!()
    }
}

impl<InstrumentKey, MarketState> OrderManager<InstrumentKey>
    for Instruments<InstrumentKey, MarketState>
where
    InstrumentKey: Debug + Clone + PartialEq + Eq + Hash,
{
    fn update_from_order(&mut self, snapshot: Snapshot<&Order<InstrumentKey, ExchangeOrderState>>) {
        let Some(state) = self.state_mut(&snapshot.0.instrument) else {
            warn!(
                instrument_id = ?snapshot.0.instrument,
                event = ?snapshot,
                "OrderManager ignoring Snapshot<Order> received for non-configured instrument"
            );
            return;
        };

        state.orders.update_from_order(snapshot);
    }

    fn orders<'a>(&'a self) -> impl Iterator<Item = &'a Order<InstrumentKey, InternalOrderState>>
    where
        InstrumentKey: 'a,
    {
        self.0.values().flat_map(|state| state.orders.orders())
    }

    fn record_in_flight_cancel(&mut self, request: &Order<InstrumentKey, RequestCancel>) {
        let state = self.state_mut(&request.instrument).unwrap_or_else(|| {
            panic!(
                "OrderManager cannot record in flight Order<RequestCancel> for non-configured instrument: {:?}",
                request.instrument
            )
        });

        state.orders.record_in_flight_cancel(request)
    }

    fn record_in_flight_open(&mut self, request: &Order<InstrumentKey, RequestOpen>) {
        let state = self.state_mut(&request.instrument).unwrap_or_else(|| {
            panic!(
                "OrderManager cannot record in flight Order<RequestOpen> for non-configured instrument: {:?}",
                request.instrument
            )
        });

        state.orders.record_in_flight_open(request)
    }

    fn update_from_cancel(
        &mut self,
        response: &Order<InstrumentKey, Result<Cancelled, ExecutionError>>,
    ) {
        let Some(state) = self.state_mut(&response.instrument) else {
            warn!(
                instrument = ?response.instrument,
                event = ?response,
                "OrderManager ignoring Order<RequestCancel> response received for non-configured instrument"
            );
            return;
        };

        state.orders.update_from_cancel(response);
    }

    fn update_from_open(&mut self, response: &Order<InstrumentKey, Result<Open, ExecutionError>>) {
        let Some(state) = self.state_mut(&response.instrument) else {
            warn!(
                instrument = ?response.instrument,
                event = ?response,
                "OrderManager ignoring Order<RequestOpen> response received for non-configured instrument"
            );
            return;
        };

        state.orders.update_from_open(response);
    }
}

impl<InstrumentKey, MarketState> Instruments<InstrumentKey, MarketState>
where
    InstrumentKey: Eq + Hash,
{
    pub fn state(
        &self,
        instrument: &InstrumentKey,
    ) -> Option<&InstrumentState<InstrumentKey, MarketState>> {
        self.0.get(instrument)
    }
    pub fn state_mut(
        &mut self,
        instrument: &InstrumentKey,
    ) -> Option<&mut InstrumentState<InstrumentKey, MarketState>> {
        self.0.get_mut(instrument)
    }
}

impl<InstrumentKey, MarketState> FromIterator<InstrumentState<InstrumentKey, MarketState>>
    for Instruments<InstrumentKey, MarketState>
where
    InstrumentKey: Clone + Hash + Eq,
{
    fn from_iter<T: IntoIterator<Item = InstrumentState<InstrumentKey, MarketState>>>(
        iter: T,
    ) -> Self {
        Instruments(
            iter.into_iter()
                .map(|state| (state.instrument.id.clone(), state))
                .collect(),
        )
    }
}
