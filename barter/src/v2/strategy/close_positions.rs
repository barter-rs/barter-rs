use crate::v2::{
    engine::state::{
        asset::AssetStates,
        instrument::{manager::InstrumentFilter, InstrumentStates},
    },
    order::{Order, RequestCancel, RequestOpen},
    strategy::Strategy,
};

pub trait ClosePositionsStrategy<MarketState, ExchangeKey, AssetKey, InstrumentKey>
where
    Self: Strategy,
{
    fn close_positions_requests<'a>(
        &'a self,
        strategy_state: &'a Self::State,
        asset_states: &'a AssetStates,
        instrument_states: &'a InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
        filter: &'a InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>> + 'a,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>> + 'a,
    )
    where
        ExchangeKey: 'a,
        InstrumentKey: 'a;
}
