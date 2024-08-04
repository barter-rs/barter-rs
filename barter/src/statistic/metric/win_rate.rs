use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct WinRate {
    pub value: f64,
}

impl WinRate {
    pub fn calculate(wins: u64, total: u64) -> Self {
        Self {
            value: if total == 0 {
                1.0
            } else {
                wins as f64 / total as f64
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_win_rate_calculate() {
        // no trades
        assert_relative_eq!(WinRate::calculate(0, 0).value, 1.0);

        // all winning trades
        assert_relative_eq!(WinRate::calculate(10, 10).value, 1.0);

        // no winning trades
        assert_relative_eq!(WinRate::calculate(0, 10).value, 0.0);

        // mixed winning and losing trades
        assert_relative_eq!(WinRate::calculate(6, 10).value, 0.6);
    }
}
