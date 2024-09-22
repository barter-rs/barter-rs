use crate::v2::order::{Order, RequestCancel, RequestOpen};

pub mod default;

pub trait Strategy<EngineState, InstrumentKey> {
    type State;

    fn generate_orders(
        &self,
        engine_state: &EngineState,
    ) -> (
        impl IntoIterator<Item = Order<InstrumentKey, RequestCancel>>,
        impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>,
    );

    // Todo: maybe this should be feature gated, along with the Command
    //  then make trait StrategyExt?
    fn close_position_request(
        &self,
        instrument: &InstrumentKey,
        engine_state: &EngineState,
    ) -> impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>;

    // Todo: maybe this should be feature gated, along with the Command
    //  then make trait StrategyExt?
    fn close_all_positions_request(
        &self,
        engine_state: &EngineState,
    ) -> impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>;
}

// Todo: probably StrategyExt
