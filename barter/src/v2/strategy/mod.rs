use crate::v2::{
    engine::state::{asset::AssetStates, instrument::InstrumentStates},
    order::{Order, RequestCancel, RequestOpen},
};

pub mod default;

pub trait Strategy<MarketState, ExchangeKey, AssetKey, InstrumentKey> {
    type State: Clone + Send;

    fn generate_orders(
        &self,
        strategy_state: &Self::State,
        asset_states: &AssetStates,
        instrument_states: &InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    );

    fn close_position_request(
        &self,
        instrument: &InstrumentKey,
        strategy_state: &Self::State,
        asset_states: &AssetStates,
        instrument_states: &InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    );

    fn close_all_positions_request(
        &self,
        strategy_state: &Self::State,
        asset_states: &AssetStates,
        instrument_states: &InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    );
}
