use crate::v2::{
    engine::state::instrument::{market_data::MarketState, order::Orders},
    execution::error::ExecutionError,
    instrument::Instrument,
    order::{Cancelled, ExchangeOrderState, Open, Order},
    position::{PortfolioId, Position},
    trade::Trade,
    Snapshot,
};
use barter_data::{
    event::{DataKind, MarketEvent},
};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::warn;
use vecmap::VecMap;
use crate::v2::order::{RequestCancel, RequestOpen};

pub mod market_data;
pub mod order;
pub mod position;

pub trait MarketDataManager<InstrumentKey> {
    fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey>);
}

pub trait OrderManager<InstrumentKey> {
    fn record_in_flight_cancel(
        &mut self,
        request: &Order<InstrumentKey, RequestCancel>,
    );
    fn record_in_flight_open(
        &mut self,
        request: &Order<InstrumentKey, RequestOpen>,
    );
    fn update_from_cancel(
        &mut self,
        response: &Order<InstrumentKey, Result<Cancelled, ExecutionError>>,
    );
    fn update_from_open(&mut self, response: &Order<InstrumentKey, Result<Open, ExecutionError>>);
    fn update_from_order_snapshot(
        &mut self,
        snapshot: &Snapshot<Order<InstrumentKey, ExchangeOrderState>>,
    );
}

pub trait PositionManager<InstrumentKey> {
    fn position(&self, instrument: &InstrumentKey) -> Option<&Position<InstrumentKey>>;
    fn positions_by_portfolio<'a>(
        &'a self,
        portfolio: PortfolioId,
    ) -> impl Iterator<Item = &'a Position<InstrumentKey>>
    where
        InstrumentKey: 'a;
    fn update_from_trade(&mut self, trade: &Trade<InstrumentKey>);
    fn update_from_position_snapshot(&mut self, snapshot: &Snapshot<Position<InstrumentKey>>);
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize, From)]
pub struct Instruments<InstrumentKey: Eq>(pub VecMap<InstrumentKey, InstrumentState<InstrumentKey>>);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentState<InstrumentKey> {
    pub instrument: Instrument<InstrumentKey>,
    pub market: MarketState,
    pub orders: Orders<InstrumentKey>,
    pub position: Position<InstrumentKey>,
}

impl<InstrumentKey> MarketDataManager<InstrumentKey> for Instruments<InstrumentKey>
where
    InstrumentKey: Debug + Eq,
{
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

impl <InstrumentKey> OrderManager<InstrumentKey> for Instruments<InstrumentKey>
where
    InstrumentKey: Debug + Clone + Eq,
{
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

    fn update_from_order_snapshot(
        &mut self,
        snapshot: &Snapshot<Order<InstrumentKey, ExchangeOrderState>>,
    ) {
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
}

impl<InstrumentKey> PositionManager<InstrumentKey> for Instruments<InstrumentKey>
where
    InstrumentKey: Debug + Eq + Clone,
{
    fn position(&self, instrument: &InstrumentKey) -> Option<&Position<InstrumentKey>> {
        self.state(instrument).map(|state| &state.position)
    }

    fn positions_by_portfolio<'a>(
        &'a self,
        portfolio: PortfolioId,
    ) -> impl Iterator<Item = &'a Position<InstrumentKey>>
    where
        InstrumentKey: 'a,
    {
        self.0.values().filter_map(move |state| {
            (state.position.portfolio == portfolio).then_some(&state.position)
        })
    }

    fn update_from_trade(&mut self, _trade: &Trade<InstrumentKey>) {
        // Todo: should Trade contain PortfolioId? Or could remove concept for now...
        todo!()
    }

    fn update_from_position_snapshot(&mut self, snapshot: &Snapshot<Position<InstrumentKey>>) {
        let Some(state) = self.state_mut(&snapshot.0.instrument) else {
            warn!(
                instrument_id = ?snapshot.0.instrument,
                event = ?snapshot,
                "OrderManager ignoring Snapshot<Position> received for non-configured instrument"
            );
            return;
        };

        state.position = snapshot.0.clone();
    }
}

impl<InstrumentKey> Instruments<InstrumentKey>
where
    InstrumentKey: Eq
{
    pub fn state(&self, instrument: &InstrumentKey) -> Option<&InstrumentState<InstrumentKey>> {
        self.0.get(instrument)
    }
    pub fn state_mut(&mut self, instrument: &InstrumentKey) -> Option<&mut InstrumentState<InstrumentKey>> {
        self.0.get_mut(instrument)
    }
}

impl<InstrumentKey> FromIterator<InstrumentState<InstrumentKey>> for Instruments<InstrumentKey>
where
    InstrumentKey: Clone + Eq,
{
    fn from_iter<T: IntoIterator<Item = InstrumentState<InstrumentKey>>>(iter: T) -> Self {
        Instruments(
            iter.into_iter()
                .map(|state| (state.instrument.id.clone(), state))
                .collect(),
        )
    }
}
