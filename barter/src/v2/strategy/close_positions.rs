use crate::v2::{
    engine::{
        action::send_requests::SendRequestsOutput,
        state::{
            asset::AssetStates,
            instrument::{manager::InstrumentFilter, InstrumentStates},
        },
    },
    order::{Order, RequestCancel, RequestOpen},
    strategy::Strategy,
};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct ClosePositionsOutput<ExchangeKey, InstrumentKey> {
    pub cancels: SendRequestsOutput<ExchangeKey, InstrumentKey, RequestCancel>,
    pub opens: SendRequestsOutput<ExchangeKey, InstrumentKey, RequestOpen>,
}
