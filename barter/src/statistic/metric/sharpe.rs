use crate::statistic::time::TimeInterval;
use rust_decimal::prelude::Zero;
use serde::{Deserialize, Serialize};

/// Represents a Sharpe Ratio value over a specific [`TimeInterval`].
///
/// Sharpe Ratio measures the risk-adjusted return of an investment by comparing
/// its excess returns (over risk-free rate) to its standard deviation.
///
/// See docs: <https://www.investopedia.com/articles/07/sharpe_ratio.asp>
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct SharpeRatio<Interval> {
    pub value: f64,
    pub interval: Interval,
}

impl<Interval> SharpeRatio<Interval>
where
    Interval: TimeInterval,
{
    /// Calculate the [`SharpeRatio`] over the provided [`TimeInterval`].
    pub fn calculate(
        risk_free_return: f64,
        mean_return: f64,
        std_dev_returns: f64,
        returns_period: Interval,
    ) -> Self {
        if std_dev_returns.is_zero() {
            Self {
                value: f64::INFINITY,
                interval: returns_period,
            }
        } else {
            Self {
                value: (mean_return - risk_free_return) / std_dev_returns,
                interval: returns_period,
            }
        }
    }

    /// Scale the [`SharpeRatio`] from the current [`TimeInterval`] to the provided [`TimeInterval`].
    ///
    /// This scaling assumed the returns are independently and identically distributed (IID).
    pub fn scale<TargetInterval>(self, target: TargetInterval) -> SharpeRatio<TargetInterval>
    where
        TargetInterval: TimeInterval,
    {
        // Determine scale factor: square root of number of Self Intervals in TargetIntervals
        let scale = (target.interval().num_seconds() as f64
            / self.interval.interval().num_seconds() as f64)
            .sqrt();

        SharpeRatio {
            value: self.value * scale,
            interval: target,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistic::time::{Annual252, Daily};
    use approx::assert_relative_eq;
    use chrono::TimeDelta;

    #[test]
    fn test_sharpe_ratio_with_zero_std_dev() {
        let risk_free_return = 0.001;
        let mean_return = 0.002;
        let std_dev_returns = 0.0;
        let time_period = TimeDelta::hours(2);

        let result =
            SharpeRatio::calculate(risk_free_return, mean_return, std_dev_returns, time_period);
        assert!(result.value.is_infinite());
    }

    #[test]
    fn test_sharpe_ratio_calculate_with_custom_interval() {
        // Define custom interval returns statistics
        let risk_free_return = 0.0015; // 0.15%
        let mean_return = 0.0025; // 0.25%
        let std_dev_returns = 0.02; // 2%
        let time_period = TimeDelta::hours(2);

        let actual =
            SharpeRatio::calculate(risk_free_return, mean_return, std_dev_returns, time_period);

        let expected = SharpeRatio {
            value: 0.05,
            interval: time_period,
        };

        assert_relative_eq!(actual.value, expected.value, epsilon = 1e-4);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sharpe_ratio_calculate_with_daily_interval() {
        // Define daily returns statistics
        let risk_free_return = 0.0015; // 0.15%
        let mean_return = 0.0025; // 0.25%
        let std_dev_returns = 0.02; // 2%
        let time_period = Daily;

        let actual =
            SharpeRatio::calculate(risk_free_return, mean_return, std_dev_returns, time_period);

        let expected = SharpeRatio {
            value: 0.05,
            interval: time_period,
        };

        assert_relative_eq!(actual.value, expected.value, epsilon = 1e-4);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sharpe_ratio_scale_from_daily_to_annual_252() {
        let input = SharpeRatio {
            value: 0.05,
            interval: Daily,
        };

        let actual = input.scale(Annual252);

        let expected = SharpeRatio {
            value: 0.7937,
            interval: Annual252,
        };

        assert_relative_eq!(actual.value, expected.value, epsilon = 1e-4);
        assert_eq!(actual.interval, expected.interval);
    }
}
