use serde::{Deserialize, Serialize};
use tracing::info;

/// Represents the current `TradingState` of the `Engine`.
///
/// If `TradingState::Enabled`, the Engine will generate algorithmic orders using the
/// `AlgoStrategy` implementation.
///
/// If `TradingState::Disabled`, the Engine will continue to update it's state based on input
/// events, but it will not generate algorithmic orders. Whilst in this state, `Commands` will
/// still be actioned (such as 'open order', 'cancel order', 'close positions', etc.).
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize,
)]
pub enum TradingState {
    Enabled,
    #[default]
    Disabled,
}

impl TradingState {
    /// Updates the Engine `TradingState`.
    ///
    /// Returns a [`TradingStateUpdateAudit`] which contains a record of the previous and new
    /// state.
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase {
        name: &'static str,
        initial: TradingState,
        update: TradingState,
        expected_state: TradingState,
        expected_audit: TradingStateUpdateAudit,
    }

    #[test]
    fn test_trading_state_update() {
        let test_cases = vec![
            TestCase {
                name: "Enable when disabled",
                initial: TradingState::Disabled,
                update: TradingState::Enabled,
                expected_state: TradingState::Enabled,
                expected_audit: TradingStateUpdateAudit {
                    prev: TradingState::Disabled,
                    current: TradingState::Enabled,
                },
            },
            TestCase {
                name: "Disable when enabled",
                initial: TradingState::Enabled,
                update: TradingState::Disabled,
                expected_state: TradingState::Disabled,
                expected_audit: TradingStateUpdateAudit {
                    prev: TradingState::Enabled,
                    current: TradingState::Disabled,
                },
            },
            TestCase {
                name: "Enable when already enabled",
                initial: TradingState::Enabled,
                update: TradingState::Enabled,
                expected_state: TradingState::Enabled,
                expected_audit: TradingStateUpdateAudit {
                    prev: TradingState::Enabled,
                    current: TradingState::Enabled,
                },
            },
            TestCase {
                name: "Disable when already disabled",
                initial: TradingState::Disabled,
                update: TradingState::Disabled,
                expected_state: TradingState::Disabled,
                expected_audit: TradingStateUpdateAudit {
                    prev: TradingState::Disabled,
                    current: TradingState::Disabled,
                },
            },
        ];

        for test in test_cases {
            let mut state = test.initial;
            let audit = state.update(test.update);

            assert_eq!(
                state, test.expected_state,
                "Failed test '{}': state mismatch",
                test.name
            );

            assert_eq!(
                audit.prev, test.expected_audit.prev,
                "Failed test '{}': audit prev state mismatch",
                test.name
            );

            assert_eq!(
                audit.current, test.expected_audit.current,
                "Failed test '{}': audit current state mismatch",
                test.name
            );
        }
    }

    #[test]
    fn test_trading_state_update_audit_transition_to_disabled() {
        let test_cases = vec![
            TestCase {
                name: "Detect transition to disabled from enabled",
                initial: TradingState::Enabled,
                update: TradingState::Disabled,
                expected_state: TradingState::Disabled,
                expected_audit: TradingStateUpdateAudit {
                    prev: TradingState::Enabled,
                    current: TradingState::Disabled,
                },
            },
            TestCase {
                name: "No transition detected when already disabled",
                initial: TradingState::Disabled,
                update: TradingState::Disabled,
                expected_state: TradingState::Disabled,
                expected_audit: TradingStateUpdateAudit {
                    prev: TradingState::Disabled,
                    current: TradingState::Disabled,
                },
            },
            TestCase {
                name: "No transition detected when enabling",
                initial: TradingState::Disabled,
                update: TradingState::Enabled,
                expected_state: TradingState::Enabled,
                expected_audit: TradingStateUpdateAudit {
                    prev: TradingState::Disabled,
                    current: TradingState::Enabled,
                },
            },
        ];

        for test in test_cases {
            let mut state = test.initial;
            let audit = state.update(test.update);

            let expected_transition =
                audit.prev != TradingState::Disabled && audit.current == TradingState::Disabled;

            assert_eq!(
                audit.transitioned_to_disabled(),
                expected_transition,
                "Failed test '{}': transition detection incorrect",
                test.name
            );
        }
    }
}
