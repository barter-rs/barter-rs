use crate::v2::{
    engine::{state::EngineState, Processor},
    execution::{AccountEvent, AccountEventKind},
    order::{Order, RequestCancel, RequestOpen},
    risk::{RiskApproved, RiskManager, RiskRefused},
    strategy::default::DefaultStrategyState,
};
use barter_data::event::MarketEvent;

/// Example [`RiskManager`] implementation that approves all order requests.
///
/// *EXAMPLE IMPLEMENTATION ONLY, PLEASE DO NOT USE FOR ANYTHING OTHER THAN TESTING PURPOSES.*
#[derive(Debug, Clone)]
pub struct DefaultRiskManager;

impl<InstrumentState, BalanceState, AssetKey, InstrumentKey>
    RiskManager<InstrumentState, BalanceState, AssetKey, InstrumentKey> for DefaultRiskManager
{
    type State = DefaultRiskManagerState;
    type StrategyState = DefaultStrategyState;

    fn check(
        &self,
        _: &EngineState<
            InstrumentState,
            BalanceState,
            Self::StrategyState,
            Self::State,
            AssetKey,
            InstrumentKey,
        >,
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
