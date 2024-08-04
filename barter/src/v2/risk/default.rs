use crate::v2::{
    engine::{error::EngineError, state::DefaultEngineState},
    order::{Order, RequestCancel, RequestOpen},
    risk::{RiskApproved, RiskManager, RiskRefused},
    EngineEvent, TryUpdater,
};
use barter_data::instrument::InstrumentId;

/// Example [`RiskManager`] implementation that approves all order requests.
///
/// *EXAMPLE IMPLEMENTATION ONLY, PLEASE DO NOT USE FOR ANYTHING OTHER THAN TESTING PURPOSES.*
#[derive(Debug, Clone)]
pub struct DefaultRiskManager;

impl<StrategyState> RiskManager<DefaultEngineState<StrategyState, DefaultRiskManagerState>>
    for DefaultRiskManager
{
    type Event = EngineEvent;
    type State = DefaultRiskManagerState;

    fn approve_orders(
        &self,
        _: &DefaultEngineState<StrategyState, DefaultRiskManagerState>,
        cancels: impl IntoIterator<Item = Order<InstrumentId, RequestCancel>>,
        opens: impl IntoIterator<Item = Order<InstrumentId, RequestOpen>>,
    ) -> (
        impl Iterator<Item = RiskApproved<Order<InstrumentId, RequestCancel>>>,
        impl Iterator<Item = RiskApproved<Order<InstrumentId, RequestOpen>>>,
        impl Iterator<Item = RiskRefused<Order<InstrumentId, RequestCancel>>>,
        impl Iterator<Item = RiskRefused<Order<InstrumentId, RequestOpen>>>,
    ) {
        (
            cancels.into_iter().map(RiskApproved::new),
            opens.into_iter().map(RiskApproved::new),
            std::iter::empty(),
            std::iter::empty(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct DefaultRiskManagerState;

impl TryUpdater<&EngineEvent> for DefaultRiskManagerState {
    type Error = EngineError;

    fn try_update(&mut self, _: &EngineEvent) -> Result<(), Self::Error> {
        Ok(())
    }
}
