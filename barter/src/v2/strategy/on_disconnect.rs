use crate::v2::{engine::Engine, strategy::Strategy};
use barter_instrument::exchange::ExchangeId;

pub trait OnDisconnectStrategy<State, ExecutionTxs, Risk>
where
    Self: Strategy + Sized,
{
    type Output;
    fn on_disconnect(
        engine: &mut Engine<State, ExecutionTxs, Self, Risk>,
        exchange: ExchangeId,
    ) -> Self::Output;
}
