use crate::v2::{
    engine::{
        state::{asset::AssetStates, instrument::InstrumentStates},
        Processor,
    },
    execution::AccountEvent,
    order::{Order, RequestCancel, RequestOpen},
    risk::{RiskApproved, RiskManager, RiskRefused},
};
use barter_data::event::MarketEvent;
use std::hash::Hash;

/// Example [`RiskManager`] implementation that approves all order requests.
///
/// *EXAMPLE IMPLEMENTATION ONLY, PLEASE DO NOT USE FOR ANYTHING OTHER THAN TESTING PURPOSES.*
#[derive(Debug, Clone)]
pub struct DefaultRiskManager;

impl<MarketState, ExchangeKey, AssetKey, InstrumentKey>
    RiskManager<MarketState, ExchangeKey, AssetKey, InstrumentKey> for DefaultRiskManager
where
    AssetKey: Eq + Hash,
{
    type State = DefaultRiskManagerState;

    fn check(
        &self,
        _: &Self::State,
        _: &AssetStates,
        _: &InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
        cancels: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        opens: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskApproved<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
        impl IntoIterator<Item = RiskRefused<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskRefused<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
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

impl<ExchangeKey, AssetKey, InstrumentKey>
    Processor<&AccountEvent<ExchangeKey, AssetKey, InstrumentKey>> for DefaultRiskManagerState
{
    type Output = ();
    fn process(&mut self, _: &AccountEvent<ExchangeKey, AssetKey, InstrumentKey>) -> Self::Output {}
}

impl<InstrumentKey, Kind> Processor<&MarketEvent<InstrumentKey, Kind>> for DefaultRiskManagerState {
    type Output = ();
    fn process(&mut self, _: &MarketEvent<InstrumentKey, Kind>) -> Self::Output {}
}
