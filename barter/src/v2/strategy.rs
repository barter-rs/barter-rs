use crate::v2::{
    engine::{error::EngineError, state::DefaultEngineState},
    order::{Order, RequestCancel, RequestOpen},
    EngineEvent, StateUpdater,
};
use std::fmt::Debug;

pub trait Strategy<EngineState> {
    type Event;
    type State: for<'a> StateUpdater<&'a Self::Event> + Debug + Clone;

    fn generate_orders<InstrumentKey>(
        &self,
        engine_state: &EngineState,
    ) -> (
        impl Iterator<Item = Order<InstrumentKey, RequestCancel>>,
        impl Iterator<Item = Order<InstrumentKey, RequestOpen>>,
    );
}

#[derive(Debug, Clone)]
pub struct DefaultStrategy;

impl<RiskState> Strategy<DefaultEngineState<DefaultStrategyState, RiskState>> for DefaultStrategy {
    type Event = EngineEvent;
    type State = DefaultStrategyState;

    fn generate_orders<InstrumentKey>(
        &self,
        _: &DefaultEngineState<Self::State, RiskState>,
    ) -> (
        impl Iterator<Item = Order<InstrumentKey, RequestCancel>>,
        impl Iterator<Item = Order<InstrumentKey, RequestOpen>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }
}

#[derive(Debug, Clone)]
pub struct DefaultStrategyState;

impl StateUpdater<&EngineEvent> for DefaultStrategyState {
    type Output = ();
    type Error = EngineError;

    fn try_update(&mut self, _: &EngineEvent) -> Result<(), Self::Error> {
        Ok(())
    }
}
