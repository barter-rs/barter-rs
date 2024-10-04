use crate::v2::order::{ExchangeOrderState, InternalOrderState, RequestCancel, RequestOpen};
use crate::v2::{engine::state::instrument::{market_data::MarketState, order::Orders}, execution::error::ExecutionError, instrument::Instrument, order::{Cancelled, Open, Order}, trade::Trade, Snapshot};
use barter_data::event::{DataKind, MarketEvent};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;
use fnv::FnvHashMap;
use tracing::warn;
use position::PositionManager;
use crate::v2::engine::state::instrument::market_data::MarketDataManager;
use crate::v2::engine::state::instrument::order::OrderManager;
use crate::v2::execution::{InstrumentAccountSnapshot};

pub mod market_data;
pub mod order;
pub mod position;

pub trait InstrumentStateManager<InstrumentKey>: Clone {
    fn update_from_snapshot(&mut self, snapshot: &[InstrumentAccountSnapshot<InstrumentKey>]);
    fn market_data(&self) -> &impl MarketDataManager<InstrumentKey>;
    fn market_data_mut(&mut self) -> &mut impl MarketDataManager<InstrumentKey>;
    fn orders(&self) -> &impl OrderManager<InstrumentKey>;
    fn orders_mut(&mut self) -> &mut impl OrderManager<InstrumentKey>;
    fn positions(&self) -> &impl PositionManager<InstrumentKey>;
    fn positions_mut(&mut self) -> &mut impl PositionManager<InstrumentKey>;
}


#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize, From)]
pub struct Instruments<InstrumentKey: Eq + Hash>(
    pub FnvHashMap<InstrumentKey, InstrumentState<InstrumentKey>>,
);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentState<InstrumentKey> {
    pub instrument: Instrument<InstrumentKey>,
    pub market: MarketState,
    pub orders: Orders<InstrumentKey>,
    pub position: Position<InstrumentKey>,
}

impl<InstrumentKey> InstrumentStateManager<InstrumentKey> for Instruments<InstrumentKey>
where
    InstrumentKey: Debug + Clone + Eq + Hash,
{
    fn update_from_snapshot(&mut self, snapshot: &[InstrumentAccountSnapshot<InstrumentKey>]) {
        todo!()
    }

    fn market_data(&self) -> &impl MarketDataManager<InstrumentKey> {
        self
    }

    fn market_data_mut(&mut self) -> &mut impl MarketDataManager<InstrumentKey> {
        self
    }

    fn orders(&self) -> &impl OrderManager<InstrumentKey> {
        self
    }

    fn orders_mut(&mut self) -> &mut impl OrderManager<InstrumentKey> {
        self
    }

    fn positions(&self) -> &impl PositionManager<InstrumentKey> {
        self
    }

    fn positions_mut(&mut self) -> &mut impl PositionManager<InstrumentKey> {
        self
    }
}

impl<InstrumentKey> MarketDataManager<InstrumentKey> for Instruments<InstrumentKey>
where
    InstrumentKey: Debug + Clone + Eq + Hash,
{
    fn update_from_snapshot(&mut self, snapshot: Snapshot<Self>) {
        todo!()
    }

    fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey>) {
        let Some(state) = self.state_mut(&event.instrument) else {
            warn!(
                exchange = %event.exchange,
                instrument = ?event.instrument,
                ?event,
                "MarketDataManager ignoring MarketEvent received for non-configured instrument",
            );
            return;
        };

        if let DataKind::OrderBookL1(l1) = event.kind {
            state.market.update_from_l1(l1)
        } else {
            warn!(
                ?event,
                supported = "[OrderBookL1]",
                "MarketDataManager ignoring MarketEvent since it's Kind is not supported"
            );
        }
    }
}


impl<InstrumentKey> OrderManager<InstrumentKey> for Instruments<InstrumentKey>
where
    InstrumentKey: Debug + Clone + PartialEq + Eq + Hash,
{
    fn update_from_orders_snapshot(&mut self, snapshot: Snapshot<&Vec<Order<InstrumentKey, Open>>>) {
        todo!()
    }

    fn update_from_order_snapshot(&mut self, snapshot: Snapshot<&Order<InstrumentKey, ExchangeOrderState>>) {
        let Some(state) = self.state_mut(&snapshot.0.instrument) else {
            warn!(
                instrument_id = ?snapshot.0.instrument,
                event = ?snapshot,
                "OrderManager ignoring Snapshot<Order> received for non-configured instrument"
            );
            return;
        };

        state.orders.update_from_order_snapshot(snapshot);
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

impl<InstrumentKey> Instruments<InstrumentKey>
where
    InstrumentKey: Eq + Hash,
{
    pub fn state(&self, instrument: &InstrumentKey) -> Option<&InstrumentState<InstrumentKey>> {
        self.0.get(instrument)
    }
    pub fn state_mut(
        &mut self,
        instrument: &InstrumentKey,
    ) -> Option<&mut InstrumentState<InstrumentKey>> {
        self.0.get_mut(instrument)
    }
}

impl<InstrumentKey> FromIterator<InstrumentState<InstrumentKey>> for Instruments<InstrumentKey>
where
    InstrumentKey: Clone + Hash + Eq,
{
    fn from_iter<T: IntoIterator<Item = InstrumentState<InstrumentKey>>>(iter: T) -> Self {
        Instruments(
            iter.into_iter()
                .map(|state| (state.instrument.id.clone(), state))
                .collect(),
        )
    }
}
