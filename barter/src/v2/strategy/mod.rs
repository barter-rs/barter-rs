
use crate::v2::{
    order::{Order, RequestCancel, RequestOpen},
};
use std::fmt::Debug;
use crate::v2::engine::Processor;

pub trait Strategy<EngineState, InstrumentKey> {
    type Event;
    type State: for<'a> Processor<&'a Self::Event> + Debug + Clone;

    fn generate_orders(
        &self,
        engine_state: &EngineState,
    ) -> (
        impl Iterator<Item = Order<InstrumentKey, RequestCancel>>,
        impl Iterator<Item = Order<InstrumentKey, RequestOpen>>,
    );
}

