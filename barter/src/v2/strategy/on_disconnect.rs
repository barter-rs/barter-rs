use crate::v2::engine::Engine;
use barter_instrument::exchange::ExchangeId;

pub trait OnDisconnectStrategy<State, ExecutionTxs, Risk>
where
    Self: Sized,
{
    type OnDisconnect;

    fn on_disconnect(
        engine: &mut Engine<State, ExecutionTxs, Self, Risk>,
        exchange: ExchangeId,
    ) -> Self::OnDisconnect;
}
