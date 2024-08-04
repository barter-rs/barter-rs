use crate::v2::{
    engine::{error::EngineError, state::DefaultEngineState},
    order::{Order, RequestCancel, RequestOpen},
    EngineEvent, TryUpdater,
};
use barter_data::instrument::InstrumentId;
use std::fmt::Debug;

pub trait Strategy<EngineState> {
    type Event;
    type State: for<'a> TryUpdater<&'a Self::Event> + Debug + Clone;

    fn generate_orders(
        &self,
        engine_state: &EngineState,
    ) -> (
        impl Iterator<Item = Order<InstrumentId, RequestCancel>>,
        impl Iterator<Item = Order<InstrumentId, RequestOpen>>,
    );
}

#[derive(Debug, Clone)]
pub struct DefaultStrategy;

impl<RiskState> Strategy<DefaultEngineState<DefaultStrategyState, RiskState>> for DefaultStrategy {
    type Event = EngineEvent;
    type State = DefaultStrategyState;

    fn generate_orders(
        &self,
        _: &DefaultEngineState<Self::State, RiskState>,
    ) -> (
        impl Iterator<Item = Order<InstrumentId, RequestCancel>>,
        impl Iterator<Item = Order<InstrumentId, RequestOpen>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }
}

#[derive(Debug, Clone)]
pub struct DefaultStrategyState;

impl TryUpdater<&EngineEvent> for DefaultStrategyState {
    type Error = EngineError;

    fn try_update(&mut self, _: &EngineEvent) -> Result<(), Self::Error> {
        Ok(())
    }
}
