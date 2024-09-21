use crate::Strategy;

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

impl Processor<&EngineEvent> for DefaultStrategyState {
    type Output = Result<(), EngineError>;

    fn process(&mut self, _: &EngineEvent) -> Self::Output {
        Ok(())
    }
}