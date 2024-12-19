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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    #[derive(Debug)]
    struct MockTradingStateManager {
        trading: TradingState,
    }

    impl MockTradingStateManager {
        fn new(initial_state: TradingState) -> Self {
            Self {
                trading: initial_state,
            }
        }
    }

    impl TradingStateManager for MockTradingStateManager {
        fn trading(&self) -> TradingState {
            self.trading
        }

        fn update_trading_state(&mut self, update: TradingState) -> TradingStateUpdateAudit {
            let prev = self.trading;
            self.trading = update;
            TradingStateUpdateAudit {
                prev,
                current: update,
            }
        }
    }

    #[test]
    fn test_initial_trading_state() {
        let state = MockEngineState::new(TradingState::Enabled);
        assert_eq!(state.trading(), TradingState::Enabled);

        let state = MockEngineState::new(TradingState::Disabled);
        assert_eq!(state.trading(), TradingState::Disabled);
    }

    #[test]
    fn test_update_trading_state() {
        let mut state = MockEngineState::new(TradingState::Enabled);

        // Test transition to disabled
        let audit = state.update_trading_state(TradingState::Disabled);
        assert_eq!(audit.prev, TradingState::Enabled);
        assert_eq!(audit.current, TradingState::Disabled);
        assert_eq!(state.trading(), TradingState::Disabled);

        // Test transition back to enabled
        let audit = state.update_trading_state(TradingState::Enabled);
        assert_eq!(audit.prev, TradingState::Disabled);
        assert_eq!(audit.current, TradingState::Enabled);
        assert_eq!(state.trading(), TradingState::Enabled);
    }

    #[test]
    fn test_redundant_state_updates() {
        let mut state = MockEngineState::new(TradingState::Enabled);

        // Test setting enabled when already enabled
        let audit = state.update_trading_state(TradingState::Enabled);
        assert_eq!(audit.prev, TradingState::Enabled);
        assert_eq!(audit.current, TradingState::Enabled);
        assert!(!audit.transitioned_to_disabled());

        // Set to disabled
        state.update_trading_state(TradingState::Disabled);

        // Test setting disabled when already disabled
        let audit = state.update_trading_state(TradingState::Disabled);
        assert_eq!(audit.prev, TradingState::Disabled);
        assert_eq!(audit.current, TradingState::Disabled);
        assert!(!audit.transitioned_to_disabled());
    }

    #[test]
    fn test_transitioned_to_disabled() {
        // Test transition from enabled to disabled
        let audit = TradingStateUpdateAudit {
            prev: TradingState::Enabled,
            current: TradingState::Disabled,
        };
        assert!(audit.transitioned_to_disabled());

        // Test already disabled
        let audit = TradingStateUpdateAudit {
            prev: TradingState::Disabled,
            current: TradingState::Disabled,
        };
        assert!(!audit.transitioned_to_disabled());

        // Test enabled to enabled
        let audit = TradingStateUpdateAudit {
            prev: TradingState::Enabled,
            current: TradingState::Enabled,
        };
        assert!(!audit.transitioned_to_disabled());

        // Test disabled to enabled
        let audit = TradingStateUpdateAudit {
            prev: TradingState::Disabled,
            current: TradingState::Enabled,
        };
        assert!(!audit.transitioned_to_disabled());
    }
}
