use crate::statistic::time::TimeInterval;
use rust_decimal::prelude::Zero;
use serde::{Deserialize, Serialize};

/// Represents a Sortino Ratio value over a specific [`TimeInterval`].
///
/// Similar to the Sharpe Ratio, but only considers downside volatility (standard deviation of
/// negative returns) rather than total volatility. This makes it a better metric for portfolios
/// with non-normal return distributions.
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct SortinoRatio<Interval> {
    pub value: f64,
    pub interval: Interval,
}

impl<Interval> SortinoRatio<Interval>
where
    Interval: TimeInterval,
{
    /// Calculate the [`SortinoRatio`] over the provided [`TimeInterval`].
    pub fn calculate(
        risk_free_return: f64,
        mean_return: f64,
        std_dev_loss_returns: f64,
        returns_period: Interval,
    ) -> Self {
        if std_dev_loss_returns.is_zero() {
            Self {
                value: if mean_return > risk_free_return {
                    // Special case: +ve excess returns with no downside risk (very good)
                    f64::INFINITY
                } else if mean_return < risk_free_return {
                    // Special case: -ve excess returns with no downside risk (very bad)
                    f64::NEG_INFINITY
                } else {
                    // Special case: no excess returns with no downside risk (neutral)
                    0.0
                },
                interval: returns_period,
            }
        } else {
            Self {
                value: (mean_return - risk_free_return) / std_dev_loss_returns,
                interval: returns_period,
            }
        }
    }

    /// Scale the [`SortinoRatio`] from the current [`TimeInterval`] to the provided [`TimeInterval`].
    ///
    /// This scaling assumed the returns are independently and identically distributed (IID).
    /// However, this assumption may be less appropriate for downside deviation.
    pub fn scale<TargetInterval>(self, target: TargetInterval) -> SortinoRatio<TargetInterval>
    where
        TargetInterval: TimeInterval,
    {
        // Determine scale factor: square root of number of Self Intervals in TargetIntervals
        let scale = (target.interval().num_seconds() as f64
            / self.interval.interval().num_seconds() as f64)
            .sqrt();

        SortinoRatio {
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
    fn test_sortino_ratio_normal_case() {
        // Define test case with reasonable values
        let risk_free_return = 0.0015; // 0.15%
        let mean_return = 0.0025; // 0.25%
        let std_dev_loss_returns = 0.02; // 2%
        let time_period = Daily;

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        let expected = SortinoRatio {
            value: 0.05, // (0.0025 - 0.0015) / 0.02
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_sortino_ratio_zero_downside_dev_positive_excess() {
        // Test case: positive excess returns with no downside risk
        let risk_free_return = 0.001; // 0.1%
        let mean_return = 0.002; // 0.2%
        let std_dev_loss_returns = 0.0;
        let time_period = Daily;

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        assert!(actual.value.is_infinite() && actual.value.is_sign_positive());
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_sortino_ratio_zero_downside_dev_negative_excess() {
        // Test case: negative excess returns with no downside risk
        let risk_free_return = 0.002; // 0.2%
        let mean_return = 0.001; // 0.1%
        let std_dev_loss_returns = 0.0;
        let time_period = Daily;

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        assert!(actual.value.is_infinite() && actual.value.is_sign_negative());
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_sortino_ratio_zero_downside_dev_no_excess() {
        // Test case: no excess returns with no downside risk
        let risk_free_return = 0.001; // 0.1%
        let mean_return = 0.001; // 0.1%
        let std_dev_loss_returns = 0.0;
        let time_period = Daily;

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        assert_eq!(actual.value, 0.0);
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_sortino_ratio_negative_returns() {
        // Test case: negative mean returns
        let risk_free_return = 0.001; // 0.1%
        let mean_return = -0.002; // -0.2%
        let std_dev_loss_returns = 0.015; // 1.5%
        let time_period = Daily;

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        let expected = SortinoRatio {
            value: (-0.002 - 0.001) / 0.015,
            interval: time_period,
        };

        assert_eq!(actual.value, -0.2);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sortino_ratio_custom_interval() {
        // Test case with custom time interval
        let risk_free_return = 0.0015; // 0.15%
        let mean_return = 0.0025; // 0.25%
        let std_dev_loss_returns = 0.02; // 2%
        let time_period = TimeDelta::hours(4);

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        let expected = SortinoRatio {
            value: 0.05,
            interval: time_period,
        };

        assert_eq!(actual.value, 0.05);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sortino_ratio_scale_daily_to_annual() {
        // Test scaling from daily to annual
        let daily = SortinoRatio {
            value: 0.05,
            interval: Daily,
        };

        let actual = daily.scale(Annual252);

        let expected = SortinoRatio {
            value: 0.05 * (252.0_f64).sqrt(), // approximately 0.7937
            interval: Annual252,
        };

        assert_relative_eq!(actual.value, expected.value, epsilon = 1e-4);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sortino_ratio_scale_custom_intervals() {
        // Test scaling between custom intervals
        let two_hour = SortinoRatio {
            value: 0.05,
            interval: TimeDelta::hours(2),
        };

        let actual = two_hour.scale(TimeDelta::hours(8));

        let expected = SortinoRatio {
            value: 0.05 * (4.0_f64).sqrt(), // 4 = 8 hours / 2 hours
            interval: TimeDelta::hours(8),
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sortino_ratio_extreme_values() {
        // Test with very small values
        let small = SortinoRatio::calculate(1e-10, 2e-10, 1e-10, Daily);
        assert_relative_eq!(small.value, 1.0, epsilon = 1e-4);

        // Test with very large values
        let large = SortinoRatio::calculate(1e10, 2e10, 1e10, Daily);
        assert_relative_eq!(large.value, 1.0, epsilon = 1e-4);
    }
}
