use crate::v2::engine::state::{trading::TradingState, EngineState};
use tracing::info;

pub trait TradingStateManager {
    fn trading(&self) -> TradingState;
    fn update_trading_state(&mut self, update: TradingState) -> TradingStateUpdateAudit;
}

pub struct TradingStateUpdateAudit {
    pub prev: TradingState,
    pub current: TradingState,
}

impl TradingStateUpdateAudit {
    pub fn transitioned_to_disabled(&self) -> bool {
        self.current == TradingState::Disabled && self.prev != TradingState::Disabled
    }
}

impl<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey> TradingStateManager
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
{
    fn trading(&self) -> TradingState {
        self.trading
    }

    fn update_trading_state(&mut self, update: TradingState) -> TradingStateUpdateAudit {
        let prev = self.trading;
        let next = match (self.trading, update) {
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

        TradingStateUpdateAudit {
            prev,
            current: next,
        }
    }
}
