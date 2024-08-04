use crate::v2::{
    engine::state::instrument::{market_data::MarketState, order::Orders},
    execution::error::ExecutionError,
    instrument::Instrument,
    order::{Cancelled, ExchangeOrderState, Open, OpenInFlight, Order},
    position::{PortfolioId, Position},
    trade::Trade,
    Snapshot,
};
use barter_data::{
    event::{DataKind, MarketEvent},
    instrument::InstrumentId,
};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::warn;
use vecmap::VecMap;

pub mod market_data;
pub mod order;
pub mod position;

pub trait MarketDataManager<InstrumentKey> {
    fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey>);
}

pub trait OrderManager<InstrumentKey> {
    fn record_in_flights(
        &mut self,
        requests: impl IntoIterator<Item = Order<InstrumentKey, OpenInFlight>>,
    );
    fn update_from_open(&mut self, response: &Order<InstrumentKey, Result<Open, ExecutionError>>);
    fn update_from_cancel(
        &mut self,
        response: &Order<InstrumentKey, Result<Cancelled, ExecutionError>>,
    );
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
pub struct Instruments(pub VecMap<InstrumentId, InstrumentState>);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentState {
    pub instrument: Instrument,
    pub market: MarketState,
    pub orders: Orders<InstrumentId>,
    pub position: Position,
}

impl MarketDataManager<InstrumentId> for Instruments {
    fn update_from_market(&mut self, event: &MarketEvent<InstrumentId>) {
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

impl OrderManager<InstrumentId> for Instruments {
    fn record_in_flights(
        &mut self,
        requests: impl IntoIterator<Item = Order<InstrumentId, OpenInFlight>>,
    ) {
        for request in requests {
            let state = self.state_mut(&request.instrument).unwrap_or_else(|| {
                panic!(
                    "OrderManager cannot record Order<InFlight> for non-configured instrument: {:?}",
                    request.instrument
                )
            });

            state.orders.record_in_flights(std::iter::once(request))
        }
    }

    fn update_from_open(&mut self, response: &Order<InstrumentId, Result<Open, ExecutionError>>) {
        let Some(state) = self.state_mut(&response.instrument) else {
            warn!(
                instrument = %response.instrument,
                event = ?response,
                "OrderManager ignoring Order<RequestOpen> response received for non-configured instrument"
            );
            return;
        };

        state.orders.update_from_open(response);
    }

    fn update_from_cancel(
        &mut self,
        response: &Order<InstrumentId, Result<Cancelled, ExecutionError>>,
    ) {
        let Some(state) = self.state_mut(&response.instrument) else {
            warn!(
                instrument = %response.instrument,
                event = ?response,
                "OrderManager ignoring Order<RequestCancel> response received for non-configured instrument"
            );
            return;
        };

        state.orders.update_from_cancel(response);
    }

    fn update_from_order_snapshot(
        &mut self,
        snapshot: &Snapshot<Order<InstrumentId, ExchangeOrderState>>,
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

impl PositionManager<InstrumentId> for Instruments {
    fn position(&self, instrument: &InstrumentId) -> Option<&Position<InstrumentId>> {
        self.state(instrument).map(|state| &state.position)
    }

    fn positions_by_portfolio<'a>(
        &'a self,
        portfolio: PortfolioId,
    ) -> impl Iterator<Item = &'a Position<InstrumentId>>
    where
        InstrumentId: 'a,
    {
        self.0.values().filter_map(move |state| {
            if state.position.portfolio == portfolio {
                Some(&state.position)
            } else {
                None
            }
        })
    }

    fn update_from_trade(&mut self, trade: &Trade<InstrumentId>) {
        todo!()
    }

    fn update_from_position_snapshot(&mut self, snapshot: &Snapshot<Position<InstrumentId>>) {
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

impl Instruments {
    pub fn state(&self, instrument: &InstrumentId) -> Option<&InstrumentState> {
        self.0.get(instrument)
    }
    pub fn state_mut(&mut self, instrument: &InstrumentId) -> Option<&mut InstrumentState> {
        self.0.get_mut(instrument)
    }
}

impl FromIterator<InstrumentState> for Instruments {
    fn from_iter<T: IntoIterator<Item = InstrumentState>>(iter: T) -> Self {
        Instruments(
            iter.into_iter()
                .map(|state| (state.instrument.id, state))
                .collect(),
        )
    }
}
