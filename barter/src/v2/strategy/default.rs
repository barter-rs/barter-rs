use crate::v2::{
    engine::{state::EngineState, Processor},
    execution::{AccountEvent, AccountEventKind},
    order::{Order, RequestCancel, RequestOpen},
    risk::default::DefaultRiskManagerState,
    strategy::Strategy,
};
use barter_data::event::MarketEvent;

#[derive(Debug, Clone)]
pub struct DefaultStrategy;

impl<InstrumentState, BalanceState, AssetKey, InstrumentKey>
    Strategy<InstrumentState, BalanceState, AssetKey, InstrumentKey> for DefaultStrategy
{
    type State = DefaultStrategyState;
    type RiskState = DefaultRiskManagerState;

    fn generate_orders(
        &self,
        _: &EngineState<
            InstrumentState,
            BalanceState,
            Self::State,
            Self::RiskState,
            AssetKey,
            InstrumentKey,
        >,
    ) -> (
        impl IntoIterator<Item = Order<InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }

    fn close_position_request(
        &self,
        _: &InstrumentKey,
        _: &EngineState<
            InstrumentState,
            BalanceState,
            Self::State,
            Self::RiskState,
            AssetKey,
            InstrumentKey,
        >,
    ) -> impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>> {
        // Todo: I could orchestrate a OrderKind=Market to close the position
        std::iter::empty()
    }

    fn close_all_positions_request(
        &self,
        _: &EngineState<
            InstrumentState,
            BalanceState,
            Self::State,
            Self::RiskState,
            AssetKey,
            InstrumentKey,
        >,
    ) -> impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>> {
        // Todo: I could orchestrate a OrderKind=Market to close the positions
        std::iter::empty()
    }
}

#[derive(Debug, Clone)]
pub struct DefaultStrategyState;

impl<AssetKey, InstrumentKey> Processor<&AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
    for DefaultStrategyState
{
    type Output = ();
    fn process(
        &mut self,
        _: &AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>,
    ) -> Self::Output {
    }
}

impl<InstrumentKey> Processor<&MarketEvent<InstrumentKey>> for DefaultStrategyState {
    type Output = ();
    fn process(&mut self, _: &MarketEvent<InstrumentKey>) -> Self::Output {}
}

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
