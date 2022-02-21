use crate::portfolio::Balance;
use crate::portfolio::position::Position;
use crate::statistic::summary::PositionSummariser;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod drawdown;
pub mod ratio;

/// Total equity at a point in time - equates to [`Balance.total`](Balance).
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct EquityPoint {
    timestamp: DateTime<Utc>,
    total: f64,
}

impl Default for EquityPoint {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            total: 0.0,
        }
    }
}

impl From<Balance> for EquityPoint {
    fn from(balance: Balance) -> Self {
        Self {
            timestamp: balance.timestamp,
            total: balance.total
        }
    }
}

impl PositionSummariser for EquityPoint {
    /// Updates using the input [`Position`]'s PnL & associated timestamp.
    fn update(&mut self, position: &Position) {
        match position.meta.exit_balance {
            None => {
                // Position is not exited, so simulate
                self.timestamp = position.meta.last_update_timestamp;
                self.total += position.unrealised_profit_loss;
            }
            Some(exit_balance) => {
                self.timestamp = exit_balance.timestamp;
                self.total += position.realised_profit_loss;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Add;
    use chrono::Duration;
    use super::*;
    use crate::test_util::position;

    #[test]
    fn equity_point_update() {
        fn equity_update_position_closed(
            exit_timestamp: DateTime<Utc>,
            result_pnl: f64,
        ) -> Position {
            let mut position = position();
            position.meta.exit_balance = Some(Balance {
                timestamp: exit_timestamp,
                total: 100.0,
                available: 100.0
            });
            position.realised_profit_loss = result_pnl;
            position
        }

        fn equity_update_position_open(
            last_update_timestamp: DateTime<Utc>,
            unreal_pnl: f64,
        ) -> Position {
            let mut position = position();
            position.meta.exit_balance = None;
            position.meta.last_update_timestamp = last_update_timestamp;
            position.unrealised_profit_loss = unreal_pnl;
            position
        }

        struct TestCase {
            position: Position,
            expected_equity: f64,
            expected_timestamp: DateTime<Utc>,
        }

        let base_timestamp = Utc::now();

        let mut equity_point = EquityPoint {
            timestamp: base_timestamp,
            total: 100.0,
        };

        let test_cases = vec![
            TestCase {
                position: equity_update_position_closed(
                    base_timestamp.add(Duration::days(1)),
                    10.0,
                ),
                expected_equity: 110.0,
                expected_timestamp: base_timestamp.add(Duration::days(1)),
            },
            TestCase {
                position: equity_update_position_open(base_timestamp.add(Duration::days(2)), -10.0),
                expected_equity: 100.0,
                expected_timestamp: base_timestamp.add(Duration::days(2)),
            },
            TestCase {
                position: equity_update_position_closed(
                    base_timestamp.add(Duration::days(3)),
                    -55.9,
                ),
                expected_equity: 44.1,
                expected_timestamp: base_timestamp.add(Duration::days(3)),
            },
            TestCase {
                position: equity_update_position_open(base_timestamp.add(Duration::days(4)), 68.7),
                expected_equity: 112.8,
                expected_timestamp: base_timestamp.add(Duration::days(4)),
            },
            TestCase {
                position: equity_update_position_closed(
                    base_timestamp.add(Duration::days(5)),
                    99999.0,
                ),
                expected_equity: 100111.8,
                expected_timestamp: base_timestamp.add(Duration::days(5)),
            },
            TestCase {
                position: equity_update_position_open(base_timestamp.add(Duration::days(5)), 0.2),
                expected_equity: 100112.0,
                expected_timestamp: base_timestamp.add(Duration::days(5)),
            },
        ];

        for (index, test) in test_cases.into_iter().enumerate() {
            equity_point.update(&test.position);
            let equity_diff = equity_point.total - test.expected_equity;
            assert!(equity_diff < 1e-10, "Test case {} failed at assert", index);
            assert_eq!(equity_point.timestamp, test.expected_timestamp, "Test case {} failed to assert_eq", index);
        }
    }
}