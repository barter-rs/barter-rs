use crate::engine::Engine;

pub trait OnTradingDisabled<State, ExecutionTxs, Risk>
where
    Self: Sized,
{
    type OnTradingDisabled;
    fn on_trading_disabled(
        engine: &mut Engine<State, ExecutionTxs, Self, Risk>,
    ) -> Self::OnTradingDisabled;
}
