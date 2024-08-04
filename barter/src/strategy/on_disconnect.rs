use crate::engine::Engine;
use barter_instrument::exchange::ExchangeId;

pub trait OnDisconnectStrategy<Clock, State, ExecutionTxs, Risk>
where
    Self: Sized,
{
    type OnDisconnect;

    fn on_disconnect(
        engine: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
        exchange: ExchangeId,
    ) -> Self::OnDisconnect;
}
