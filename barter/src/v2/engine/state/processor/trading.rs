use crate::v2::engine::state::{trading::TradingState, EngineState, Updater};
use tracing::info;

impl<Market, Strategy, Risk> Updater<TradingState> for EngineState<Market, Strategy, Risk> {
    type Output = ();

    fn update(&mut self, event: &TradingState) -> Self::Output {
        let next = match (self.trading, event) {
            (TradingState::Enabled, TradingState::Disabled) => {
                info!("EngineState setting TradingState::Disabled");
                TradingState::Disabled
            }
            (TradingState::Disabled, TradingState::Enabled) => {
                info!("EngineState setting TradingState::Enabled");
                TradingState::Enabled
            }
            (TradingState::Enabled, TradingState::Enabled) => {
                info!("EngineState set TradingState::Enabled, although it was already enabled");
                TradingState::Enabled
            }
            (TradingState::Disabled, TradingState::Disabled) => {
                info!("EngineState set TradingState::Disabled, although it was already disabled");
                TradingState::Disabled
            }
        };

        self.trading = next;
    }
}
