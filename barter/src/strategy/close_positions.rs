use crate::engine::state::instrument::manager::InstrumentFilter;
use barter_execution::order::{Order, RequestCancel, RequestOpen};

pub trait ClosePositionsStrategy<ExchangeKey, AssetKey, InstrumentKey> {
    type State;

    fn close_positions_requests<'a>(
        &'a self,
        state: &'a Self::State,
        filter: &'a InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>> + 'a,
        impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>> + 'a,
    )
    where
        ExchangeKey: 'a,
        AssetKey: 'a,
        InstrumentKey: 'a;
}
