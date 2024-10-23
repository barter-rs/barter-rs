use crate::v2::{
    engine_new::state::{EngineState, Updater},
    execution::{AccountEvent, AccountEventKind},
    order::{Order, RequestCancel, RequestOpen},
    risk::{RiskApproved, RiskManager, RiskRefused},
};
use barter_data::event::MarketEvent;

/// Example [`RiskManager`] implementation that approves all order requests.
///
/// *EXAMPLE IMPLEMENTATION ONLY, PLEASE DO NOT USE FOR ANYTHING OTHER THAN TESTING PURPOSES.*
#[derive(Debug, Clone)]
pub struct DefaultRiskManager;

impl<MarketState, StrategyState, InstrumentKey>
    RiskManager<MarketState, StrategyState, InstrumentKey> for DefaultRiskManager
{
    type State = DefaultRiskManagerState;

    fn check(
        &self,
        _: &EngineState<MarketState, StrategyState, Self::State>,
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

impl<AssetKey, InstrumentKey> Updater<AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
    for DefaultRiskManagerState
{
    type Output = ();
    fn update(
        &mut self,
        _: &AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>,
    ) -> Self::Output {
    }
}

impl<InstrumentKey, Kind> Updater<MarketEvent<InstrumentKey, Kind>> for DefaultRiskManagerState {
    type Output = ();
    fn update(&mut self, _: &MarketEvent<InstrumentKey, Kind>) -> Self::Output {}
}

// impl<InstrumentState, BalanceState, AssetKey, InstrumentKey>
//     RiskManager<InstrumentState, BalanceState, AssetKey, InstrumentKey> for DefaultRiskManager
// {
//     type State = DefaultRiskManagerState;
//     type StrategyState = DefaultStrategyState;
//
//     fn check(
//         &self,
//         _: &EngineState<
//             InstrumentState,
//             BalanceState,
//             Self::StrategyState,
//             Self::State,
//             AssetKey,
//             InstrumentKey,
//         >,
//         cancels: impl IntoIterator<Item = Order<InstrumentKey, RequestCancel>>,
//         opens: impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>,
//     ) -> (
//         impl IntoIterator<Item = RiskApproved<Order<InstrumentKey, RequestCancel>>>,
//         impl IntoIterator<Item = RiskApproved<Order<InstrumentKey, RequestOpen>>>,
//         impl IntoIterator<Item = RiskRefused<Order<InstrumentKey, RequestCancel>>>,
//         impl IntoIterator<Item = RiskRefused<Order<InstrumentKey, RequestOpen>>>,
//     ) {
//         (
//             cancels.into_iter().map(RiskApproved::new),
//             opens.into_iter().map(RiskApproved::new),
//             std::iter::empty(),
//             std::iter::empty(),
//         )
//     }
// }
