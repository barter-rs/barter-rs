use crate::v2::{
    engine::state::{asset::AssetStates, instrument::InstrumentStates},
    order::{Order, RequestCancel, RequestOpen},
    strategy::Strategy,
};

pub trait AlgoStrategy<MarketState, ExchangeKey, AssetKey, InstrumentKey>
where
    Self: Strategy,
{
    fn generate_algo_orders(
        &self,
        strategy_state: &Self::State,
        asset_states: &AssetStates,
        instrument_states: &InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    );
}
