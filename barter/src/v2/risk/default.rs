use crate::v2::engine::Processor;
use crate::v2::execution::{AccountEvent, AccountEventKind};
use crate::v2::{
    engine::state::DefaultEngineState,
    order::{Order, RequestCancel, RequestOpen},
    risk::{RiskApproved, RiskManager, RiskRefused},
};
use barter_data::event::MarketEvent;

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

impl<AssetKey, InstrumentKey> Processor<&AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
    for DefaultRiskManagerState
{
    type Output = ();
    fn process(
        &mut self,
        _: &AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>,
    ) -> Self::Output {
    }
}

impl<InstrumentKey> Processor<&MarketEvent<InstrumentKey>> for DefaultRiskManagerState {
    type Output = ();
    fn process(&mut self, _: &MarketEvent<InstrumentKey>) -> Self::Output {}
}
