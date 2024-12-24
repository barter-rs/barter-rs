use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum TradingState {
    Enabled,
    Disabled,
}

impl TradingState {
    pub fn update(&mut self, update: TradingState) -> TradingStateUpdateAudit {
        let prev = *self;
        let next = match (*self, update) {
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

        *self = next;

        TradingStateUpdateAudit {
            prev,
            current: next,
        }
    }
}

/// Audit record of a [`TradingState`] update, containing the previous and current state.
///
/// Enables upstream components to ascertain if and how the [`TradingState`] has changed.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradingStateUpdateAudit {
    pub prev: TradingState,
    pub current: TradingState,
}

impl TradingStateUpdateAudit {
    /// Returns true only if the previous state was not `Disabled`, and the new state is.
    pub fn transitioned_to_disabled(&self) -> bool {
        self.current == TradingState::Disabled && self.prev != TradingState::Disabled
    }
}
