use crate::engine::Engine;

pub trait OnTradingDisabled<Clock, State, ExecutionTxs, Risk>
where
    Self: Sized,
{
    type OnTradingDisabled;
    fn on_trading_disabled(
        engine: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
    ) -> Self::OnTradingDisabled;
}
