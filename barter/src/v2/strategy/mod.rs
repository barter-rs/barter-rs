use crate::v2::{
    engine::state::EngineState,
    order::{Order, RequestCancel, RequestOpen},
};

pub mod default;

pub trait Strategy<InstrumentState, BalanceState, AssetKey, InstrumentKey> {
    type State;
    type RiskState;

    fn generate_orders(
        &self,
        engine_state: &EngineState<
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
    );

    // Todo: maybe this should be feature gated, along with the Command
    //  then make trait StrategyExt?
    fn close_position_request(
        &self,
        instrument: &InstrumentKey,
        engine_state: &EngineState<
            InstrumentState,
            BalanceState,
            Self::State,
            Self::RiskState,
            AssetKey,
            InstrumentKey,
        >,
    ) -> impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>;

    // Todo: maybe this should be feature gated, along with the Command
    //  then make trait StrategyExt?
    fn close_all_positions_request(
        &self,
        engine_state: &EngineState<
            InstrumentState,
            BalanceState,
            Self::State,
            Self::RiskState,
            AssetKey,
            InstrumentKey,
        >,
    ) -> impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>;
}

// Todo: probably StrategyExt
