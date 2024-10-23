use crate::v2::{
    engine::{
        state::{
            instrument::{
                market_data::MarketDataManager,
                order::{OrderManager, Orders},
            },
            UpdateFromSnapshot,
        },
        Processor,
    },
    execution::{error::ExecutionError, InstrumentAccountSnapshot},
    instrument::Instrument,
    order::{
        Cancelled, ExchangeOrderState, InternalOrderState, Open, Order, RequestCancel, RequestOpen,
    },
    position::Position,
    Snapshot,
};
use barter_data::event::MarketEvent;
use barter_instrument::Keyed;
use derive_more::{Constructor, From};
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, hash::Hash};
use tracing::warn;

pub mod market_data;
pub mod order;
pub mod position;

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize, From)]
pub struct Instruments<AssetKey, InstrumentKey: Eq + Hash, MarketState>(
    pub FnvHashMap<InstrumentKey, InstrumentState<AssetKey, InstrumentKey, MarketState>>,
);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentState<AssetKey, InstrumentKey, MarketState> {
    pub instrument: Keyed<InstrumentKey, Instrument<AssetKey>>,
    pub market: MarketState,
    pub orders: Orders<InstrumentKey>,
    pub position: Position<InstrumentKey>,
}

impl<AssetKey, InstrumentKey, MarketState>
    UpdateFromSnapshot<Vec<InstrumentAccountSnapshot<InstrumentKey>>>
    for Instruments<AssetKey, InstrumentKey, MarketState>
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
                    &mut state.orders.0,
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
                    "InstrumentState ignoring snapshot received for non-configured instrument"
                );
            }
        }
    }
}

impl<AssetKey, InstrumentKey, MarketState> MarketDataManager<InstrumentKey>
    for Instruments<AssetKey, InstrumentKey, MarketState>
where
    InstrumentKey: Debug + Eq + Hash,
    MarketState: MarketDataManager<InstrumentKey>,
{
    type Snapshot = MarketState::Snapshot;
    type MarketEventKind = MarketState::MarketEventKind;
}

impl<AssetKey, InstrumentKey, MarketState>
    Processor<&MarketEvent<InstrumentKey, MarketState::MarketEventKind>>
    for Instruments<AssetKey, InstrumentKey, MarketState>
where
    InstrumentKey: Debug + Eq + Hash,
    MarketState: MarketDataManager<InstrumentKey>,
{
    type Output = ();

    fn process(
        &mut self,
        event: &MarketEvent<InstrumentKey, MarketState::MarketEventKind>,
    ) -> Self::Output {
        let Some(state) = self.state_mut(&event.instrument) else {
            warn!(
                exchange = %event.exchange,
                instrument = ?event.instrument,
                ?event,
                "InstrumentState ignoring MarketEvent received for non-configured instrument",
            );
            return;
        };

        state.market.process(event);
    }
}

impl<AssetKey, InstrumentKey, MarketState> UpdateFromSnapshot<Vec<Order<InstrumentKey, Open>>>
    for Instruments<AssetKey, InstrumentKey, MarketState>
where
    InstrumentKey: Eq + Hash,
{
    fn update_from_snapshot(&mut self, snapshot: &Vec<Order<InstrumentKey, Open>>) {
        todo!()
    }
}

impl<AssetKey, InstrumentKey, MarketState> OrderManager<InstrumentKey>
    for Instruments<AssetKey, InstrumentKey, MarketState>
where
    InstrumentKey: Debug + Clone + PartialEq + Eq + Hash,
{
    fn update_from_snapshot(&mut self, snapshot: &Snapshot<Order<InstrumentKey, ExchangeOrderState>>) {
        let Some(state) = self.state_mut(&snapshot.0.instrument) else {
            warn!(
                instrument_id = ?snapshot.0.instrument,
                event = ?snapshot,
                "OrderManager ignoring snapshot received for non-configured instrument"
            );
            return;
        };

        state.orders.update_from_snapshot(snapshot);
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

impl<AssetKey, InstrumentKey, MarketState> Instruments<AssetKey, InstrumentKey, MarketState>
where
    InstrumentKey: Eq + Hash,
{
    pub fn state(
        &self,
        instrument: &InstrumentKey,
    ) -> Option<&InstrumentState<AssetKey, InstrumentKey, MarketState>> {
        self.0.get(instrument)
    }
    pub fn state_mut(
        &mut self,
        instrument: &InstrumentKey,
    ) -> Option<&mut InstrumentState<AssetKey, InstrumentKey, MarketState>> {
        self.0.get_mut(instrument)
    }
}

impl<AssetKey, InstrumentKey, MarketState>
    FromIterator<InstrumentState<AssetKey, InstrumentKey, MarketState>>
    for Instruments<AssetKey, InstrumentKey, MarketState>
where
    InstrumentKey: Clone + Hash + Eq,
{
    fn from_iter<T: IntoIterator<Item = InstrumentState<AssetKey, InstrumentKey, MarketState>>>(
        iter: T,
    ) -> Self {
        Instruments(
            iter.into_iter()
                .map(|state| (state.instrument.key.clone(), state))
                .collect(),
        )
    }
}
