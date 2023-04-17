use crate::{
    portfolio::{position::Position, Balance},
    statistic::summary::PositionSummariser,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod drawdown;
pub mod ratio;

/// Total equity at a point in time - equates to [`Balance.total`](Balance).
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct EquityPoint {
    pub time: DateTime<Utc>,
    pub total: f64,
}

impl Default for EquityPoint {
    fn default() -> Self {
        Self {
            time: Utc::now(),
            total: 0.0,
        }
    }
}

impl From<Balance> for EquityPoint {
    fn from(balance: Balance) -> Self {
        Self {
            time: balance.time,
            total: balance.total,
        }
    }
}

impl PositionSummariser for EquityPoint {
    /// Updates using the input [`Position`]'s PnL & associated timestamp.
    fn update(&mut self, position: &Position) {
        match position.meta.exit_balance {
            None => {
                // Position is not exited, so simulate
                self.time = position.meta.update_time;
                self.total += position.unrealised_profit_loss;
            }
            Some(exit_balance) => {
                self.time = exit_balance.time;
                self.total += position.realised_profit_loss;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::position;
    use chrono::Duration;
    use std::ops::Add;

    #[test]
    fn equity_point_update() {
        fn equity_update_position_closed(exit_time: DateTime<Utc>, result_pnl: f64) -> Position {
            let mut position = position();
            position.meta.exit_balance = Some(Balance {
                time: exit_time,
                total: 100.0,
                available: 100.0,
            });
            position.realised_profit_loss = result_pnl;
            position
        }

        fn equity_update_position_open(
            last_update_time: DateTime<Utc>,
            unreal_pnl: f64,
        ) -> Position {
            let mut position = position();
            position.meta.exit_balance = None;
            position.meta.update_time = last_update_time;
            position.unrealised_profit_loss = unreal_pnl;
            position
        }

        struct TestCase {
            position: Position,
            expected_equity: f64,
            expected_time: DateTime<Utc>,
        }

        let base_time = Utc::now();

        let mut equity_point = EquityPoint {
            time: base_time,
            total: 100.0,
        };

        let test_cases = vec![
            TestCase {
                position: equity_update_position_closed(base_time.add(Duration::days(1)), 10.0),
                expected_equity: 110.0,
                expected_time: base_time.add(Duration::days(1)),
            },
            TestCase {
                position: equity_update_position_open(base_time.add(Duration::days(2)), -10.0),
                expected_equity: 100.0,
                expected_time: base_time.add(Duration::days(2)),
            },
            TestCase {
                position: equity_update_position_closed(base_time.add(Duration::days(3)), -55.9),
                expected_equity: 44.1,
                expected_time: base_time.add(Duration::days(3)),
            },
            TestCase {
                position: equity_update_position_open(base_time.add(Duration::days(4)), 68.7),
                expected_equity: 112.8,
                expected_time: base_time.add(Duration::days(4)),
            },
            TestCase {
                position: equity_update_position_closed(base_time.add(Duration::days(5)), 99999.0),
                expected_equity: 100111.8,
                expected_time: base_time.add(Duration::days(5)),
            },
            TestCase {
                position: equity_update_position_open(base_time.add(Duration::days(5)), 0.2),
                expected_equity: 100112.0,
                expected_time: base_time.add(Duration::days(5)),
            },
        ];

        for (index, test) in test_cases.into_iter().enumerate() {
            equity_point.update(&test.position);
            let equity_diff = equity_point.total - test.expected_equity;
            assert!(equity_diff < 1e-10, "Test case {} failed at assert", index);
            assert_eq!(
                equity_point.time, test.expected_time,
                "Test case {} failed to assert_eq",
                index
            );
        }
    }
}
