use crate::statistic::summary::PositionSummariser;
use crate::portfolio::position::Position;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::portfolio::Balance;

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
    /// Updates using the input [Position]'s PnL & associated timestamp.
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