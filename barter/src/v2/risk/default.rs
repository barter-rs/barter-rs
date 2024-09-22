use crate::v2::engine::Processor;
use crate::v2::{
    engine::{error::EngineError, state::DefaultEngineState},
    order::{Order, RequestCancel, RequestOpen},
    risk::{RiskApproved, RiskManager, RiskRefused},
    EngineEvent,
};

/// Example [`RiskManager`] implementation that approves all order requests.
///
/// *EXAMPLE IMPLEMENTATION ONLY, PLEASE DO NOT USE FOR ANYTHING OTHER THAN TESTING PURPOSES.*
#[derive(Debug, Clone)]
pub struct DefaultRiskManager;

impl<AssetKey, InstrumentKey, StrategyState>
    RiskManager<
        DefaultEngineState<AssetKey, InstrumentKey, StrategyState, DefaultRiskManagerState>,
        InstrumentKey,
    > for DefaultRiskManager
where
    AssetKey: Eq,
    InstrumentKey: Eq,
{
    type State = DefaultRiskManagerState;

    fn check(
        &self,
        _: &DefaultEngineState<AssetKey, InstrumentKey, StrategyState, DefaultRiskManagerState>,
        cancels: impl IntoIterator<Item = Order<InstrumentKey, RequestCancel>>,
        opens: impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<Order<InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskApproved<Order<InstrumentKey, RequestOpen>>>,
        impl IntoIterator<Item = RiskRefused<Order<InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskRefused<Order<InstrumentKey, RequestOpen>>>,
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

impl<AssetKey, InstrumentKey> Processor<&EngineEvent<AssetKey, InstrumentKey>>
    for DefaultRiskManagerState
{
    type Output = Result<(), EngineError>;

    fn process(&mut self, _: &EngineEvent<AssetKey, InstrumentKey>) -> Self::Output {
        Ok(())
    }
}
