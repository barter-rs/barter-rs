use crate::v2::engine_new::state::{trading::TradingState, EngineState, Updater};
use tracing::info;

impl<Market, Strategy, Risk> Updater<TradingState> for EngineState<Market, Strategy, Risk> {
    type Output = ();

    fn update(&mut self, event: &TradingState) -> Self::Output {
        let next = match (self.trading, event) {
            (TradingState::Enabled, TradingState::Disabled) => {
                info!("Engine disabled trading");
                TradingState::Disabled
            }
            (TradingState::Disabled, TradingState::Enabled) => {
                info!("Engine enabled trading");
                TradingState::Enabled
            }
            (TradingState::Enabled, TradingState::Enabled) => {
                info!("Engine enabled trading, although it was already enabled");
                TradingState::Enabled
            }
            (TradingState::Disabled, TradingState::Disabled) => {
                info!("Engine disabled trading, although it was already disabled");
                TradingState::Disabled
            }
        };

        self.trading = next;
    }
}
