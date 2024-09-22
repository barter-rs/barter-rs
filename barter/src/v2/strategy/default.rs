use crate::v2::order::{Order, RequestCancel, RequestOpen};
use crate::v2::strategy::Strategy;

#[derive(Debug, Clone)]
pub struct DefaultStrategy;

impl<EngineState, InstrumentKey> Strategy<EngineState, InstrumentKey> for DefaultStrategy {
    type State = DefaultStrategyState;

    fn generate_orders(
        &self,
        _: &EngineState,
    ) -> (
        impl IntoIterator<Item = Order<InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }

    fn close_position_request(
        &self,
        _: &InstrumentKey,
        _: &EngineState,
    ) -> impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>> {
        // Todo: I could orchestrate a OrderKind=Market to close the position
        std::iter::empty()
    }

    fn close_all_positions_request(
        &self,
        _: &EngineState,
    ) -> impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>> {
        // Todo: I could orchestrate a OrderKind=Market to close the positions
        std::iter::empty()
    }
}

#[derive(Debug, Clone)]
pub struct DefaultStrategyState;

// impl Processor<&EngineEvent> for DefaultStrategyState {
//     type Output = Result<(), EngineError>;
//
//     fn process(&mut self, _: &EngineEvent) -> Self::Output {
//         Ok(())
//     }
// }

// impl<RiskState, InstrumentKey> Strategy<DefaultEngineState<DefaultStrategyState, RiskState>, InstrumentKey> for DefaultStrategy {
//     type Event = EngineEvent;
//     type State = DefaultStrategyState;
//
//     fn generate_orders(
//         &self,
//         _: &DefaultEngineState<Self::State, RiskState>,
//     ) -> (
//         impl Iterator<Item = Order<InstrumentKey, RequestCancel>>,
//         impl Iterator<Item = Order<InstrumentKey, RequestOpen>>,
//     ) {
//         (std::iter::empty(), std::iter::empty())
//     }
//
//     fn close_position_request(&self, instrument: &InstrumentKey, engine_state: &DefaultEngineState<DefaultStrategyState, RiskState>) -> impl IntoIterator<Item=Order<InstrumentKey, RequestOpen>> {
//         std::iter::empty()
//     }
//
//     fn close_all_positions_request(&self, engine_state: &DefaultEngineState<DefaultStrategyState, RiskState>) -> impl IntoIterator<Item=Order<InstrumentKey, RequestOpen>> {
//         todo!()
//     }
// }
