use crate::v2::{
    engine_new::state::order_manager::{OrderManager, Orders},
    execution::{error::ExecutionError, InstrumentAccountSnapshot},
    instrument::Instrument,
    order::{
        Cancelled, ExchangeOrderState, InternalOrderState, Open, Order, RequestCancel, RequestOpen,
    },
    position::Position,
    trade::Trade,
    Snapshot,
};
use barter_instrument::{
    asset::AssetIndex,
    instrument::{InstrumentId, InstrumentIndex},
};
use indexmap::IndexMap;

pub struct InstrumentStates<Market>(pub IndexMap<InstrumentId, InstrumentState<Market>>);

impl<Market> InstrumentStates<Market> {
    pub fn update_from_account_snapshots(
        &mut self,
        snapshots: &[InstrumentAccountSnapshot<InstrumentIndex>],
    ) {
        for snapshot in snapshots {
            let InstrumentAccountSnapshot { position, orders } = snapshot;

            let state = self.state_by_index_mut(position.instrument);

            // Replace Instrument Position
            let _ = std::mem::replace(&mut state.position, position.clone());

            // Replace Instrument orders - this wipes all open & cancel in-flight requests
            let _ = std::mem::replace(
                &mut state.orders.0,
                orders
                    .iter()
                    .map(|order| (order.cid, Order::from(order.clone())))
                    .collect(),
            );
        }
    }

    pub fn update_from_position_snapshot(
        &mut self,
        snapshot: Snapshot<&Position<InstrumentIndex>>,
    ) {
        let Snapshot(position) = snapshot;
        self.state_by_index_mut(position.instrument).position = position.clone()
    }

    pub fn update_from_trade(&mut self, _trade: &Trade<AssetIndex, InstrumentIndex>) {
        todo!()
    }

    pub fn state_by_index(&self, instrument: InstrumentIndex) -> &InstrumentState<Market> {
        self.0
            .get_index(instrument.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("InstrumentIndex: {instrument} not present in instruments"))
    }

    pub fn state_by_index_mut(
        &mut self,
        instrument: InstrumentIndex,
    ) -> &mut InstrumentState<Market> {
        self.0
            .get_index_mut(instrument.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("InstrumentIndex: {instrument} not present in instruments"))
    }
}

pub struct InstrumentState<Market> {
    pub instrument: Instrument<AssetIndex>,
    pub position: Position<InstrumentIndex>,
    pub orders: Orders<InstrumentIndex>,
    pub market: Market,
}

impl<Market> OrderManager<InstrumentIndex> for InstrumentStates<Market> {
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
