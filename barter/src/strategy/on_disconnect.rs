use crate::engine::Engine;
use barter_instrument::exchange::ExchangeId;

/// Strategy interface that defines what actions an [`Engine`] should perform after an
/// [`ExchangeId`] connection disconnects.
///
/// For example, some strategies may wish to cancel all orders, close all positions, set
/// `TradingState::Disabled`, etc.
pub trait OnDisconnectStrategy<Clock, State, ExecutionTxs, Risk>
where
    Self: Sized,
{
    /// Output of the `OnDisconnectStrategy` that is forwarded to the `AuditStream`.
    ///
    /// For example, this could include any order requests generated.
    type OnDisconnect;

    /// Perform [`Engine`] actions after receiving an [`ExchangeId`] disconnection event.
    fn on_disconnect(
        engine: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
        exchange: ExchangeId,
    ) -> Self::OnDisconnect;
}
