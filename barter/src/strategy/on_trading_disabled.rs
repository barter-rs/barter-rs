use crate::engine::Engine;

/// Strategy interface that defines what actions an [`Engine`] should perform after the
/// `TradingState` is set to `TradingState::Disabled`.
///
/// For example, some strategies may wish to cancel all orders, close all positions, etc.
pub trait OnTradingDisabled<Clock, State, ExecutionTxs, Risk>
where
    Self: Sized,
{
    /// Output of the `OnTradingDisabled` that is forwarded to the `AuditStream`.
    ///
    /// For example, this could include any order requests generated.
    type OnTradingDisabled;

    /// Perform [`Engine`] actions after the `TradingState` is set to `TradingState::Disabled`.
    fn on_trading_disabled(
        engine: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
    ) -> Self::OnTradingDisabled;
}
