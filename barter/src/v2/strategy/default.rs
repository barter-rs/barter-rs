use crate::v2::{
    engine::{
        state::{asset::AssetStates, instrument::InstrumentStates},
        Processor,
    },
    execution::AccountEvent,
    order::{Order, RequestCancel, RequestOpen},
    strategy::Strategy,
};
use barter_data::event::MarketEvent;

#[derive(Debug, Clone)]
pub struct DefaultStrategy;

impl<MarketState, ExchangeKey, AssetKey, InstrumentKey>
    Strategy<MarketState, ExchangeKey, AssetKey, InstrumentKey> for DefaultStrategy
{
    type State = DefaultStrategyState;

    fn generate_orders(
        &self,
        _: &Self::State,
        _: &AssetStates,
        _: &InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }

    fn close_position_request(
        &self,
        _: &InstrumentKey,
        _: &Self::State,
        _: &AssetStates,
        _: &InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }

    fn close_all_positions_request(
        &self,
        _: &Self::State,
        _: &AssetStates,
        _: &InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }
}

#[derive(Debug, Clone)]
pub struct DefaultStrategyState;

impl<ExchangeKey, AssetKey, InstrumentKey>
    Processor<&AccountEvent<ExchangeKey, AssetKey, InstrumentKey>> for DefaultStrategyState
{
    type Output = ();
    fn process(&mut self, _: &AccountEvent<ExchangeKey, AssetKey, InstrumentKey>) -> Self::Output {}
}

impl<InstrumentKey, Kind> Processor<&MarketEvent<InstrumentKey, Kind>> for DefaultStrategyState {
    type Output = ();
    fn process(&mut self, _: &MarketEvent<InstrumentKey, Kind>) -> Self::Output {}
}
