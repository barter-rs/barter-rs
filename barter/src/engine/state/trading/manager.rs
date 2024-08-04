use crate::engine::state::{trading::TradingState, EngineState};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Defines an interface for managing [`TradingState`] transitions, and accessing the current
/// state.
pub trait TradingStateManager {
    /// Returns the current state.
    fn trading(&self) -> TradingState;

    /// Updates the current state from an update.
    ///
    /// Note that if the update is the same as the current state an audit is still returned.
    fn update_trading_state(&mut self, update: TradingState) -> TradingStateUpdateAudit;
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
