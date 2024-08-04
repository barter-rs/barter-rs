use crate::statistic::time::TimeInterval;
use rust_decimal::prelude::Zero;
use serde::{Deserialize, Serialize};

/// Represents a Calmar Ratio value over a specific [`TimeInterval`].
///
/// The Calmar Ratio is a risk-adjusted return measure that divides the excess return
/// (over risk-free rate) by the Maximum Drawdown risk. It's similar to the Sharpe and Sortino
/// ratios, but uses Maximum Drawdown as the risk measure instead of standard deviation.
///
/// See docs: <https://corporatefinanceinstitute.com/resources/career-map/sell-side/capital-markets/calmar-ratio/>
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct CalmarRatio<Interval> {
    pub value: f64,
    pub interval: Interval,
}

impl<Interval> CalmarRatio<Interval>
where
    Interval: TimeInterval,
{
    /// Calculate the [`CalmarRatio`] over the provided [`TimeInterval`].
    pub fn calculate(
        risk_free_return: f64,
        mean_return: f64,
        max_drawdown: f64,
        returns_period: Interval,
    ) -> Self {
        if max_drawdown.is_zero() {
            Self {
                value: if mean_return > risk_free_return {
                    // Special case: +ve excess returns with no drawdown risk (very good)
                    f64::INFINITY
                } else if mean_return < risk_free_return {
                    // Special case: -ve excess returns with no drawdown risk (very bad)
                    f64::NEG_INFINITY
                } else {
                    // Special case: no excess returns with no drawdown risk (neutral)
                    0.0
                },
                interval: returns_period,
            }
        } else {
            Self {
                value: (mean_return - risk_free_return) / max_drawdown.abs(),
                interval: returns_period,
            }
        }
    }

    /// Scale the [`CalmarRatio`] from the current [`TimeInterval`] to the provided [`TimeInterval`].
    ///
    /// This scaling assumed the returns are independently and identically distributed (IID).
    /// However, this assumption is debatable since maximum drawdown may not scale with the square
    /// root of time like, for example, volatility does.
    pub fn scale<TargetInterval>(self, target: TargetInterval) -> CalmarRatio<TargetInterval>
    where
        TargetInterval: TimeInterval,
    {
        // Determine scale factor: square root of number of Self Intervals in TargetIntervals
        let scale = (target.interval().num_seconds() as f64
            / self.interval.interval().num_seconds() as f64)
            .sqrt();

        CalmarRatio {
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
    fn test_calmar_ratio_normal_case() {
        let risk_free_return = 0.0015; // 0.15%
        let mean_return = 0.0025; // 0.25%
        let max_drawdown = 0.02; // 2%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        let expected = CalmarRatio {
            value: 0.05, // (0.0025 - 0.0015) / 0.02
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_calmar_ratio_zero_drawdown_positive_excess() {
        let risk_free_return = 0.001; // 0.1%
        let mean_return = 0.002; // 0.2%
        let max_drawdown = 0.0; // 0%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        assert!(actual.value.is_infinite() && actual.value.is_sign_positive());
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_calmar_ratio_zero_drawdown_negative_excess_returns() {
        let risk_free_return = 0.002; // 0.2%
        let mean_return = 0.001; // 0.1%
        let max_drawdown = 0.0; // 0%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        assert!(actual.value.is_infinite() && actual.value.is_sign_negative());
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_calmar_ratio_zero_drawdown_negative_excess_via_negative_returns() {
        let risk_free_return = 0.002; // 0.2%
        let mean_return = -0.001; // 0.1%
        let max_drawdown = 0.0; // 0%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        assert!(actual.value.is_infinite() && actual.value.is_sign_negative());
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_calmar_ratio_zero_drawdown_no_excess_returns() {
        let risk_free_return = 0.001; // 0.1%
        let mean_return = 0.001; // 0.1%
        let max_drawdown = 0.0; // 0%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        assert_eq!(actual.value, 0.0);
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_calmar_ratio_negative_returns() {
        let risk_free_return = 0.001; // 0.1%
        let mean_return = -0.002; // -0.2%
        let max_drawdown = 0.015; // 1.5%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        let expected = CalmarRatio {
            value: (-0.002 - 0.001) / 0.015, // Should be negative
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_calmar_ratio_custom_interval() {
        let risk_free_return = 0.0015; // 0.15%
        let mean_return = 0.0025; // 0.25%
        let max_drawdown = 0.02; // 2%
        let time_period = TimeDelta::hours(4);

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        let expected = CalmarRatio {
            value: 0.05,
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_calmar_ratio_scale_daily_to_annual() {
        let daily = CalmarRatio {
            value: 0.05,
            interval: Daily,
        };

        let actual = daily.scale(Annual252);

        let expected = CalmarRatio {
            value: 0.05 * (252.0_f64).sqrt(), // approximately 0.7937
            interval: Annual252,
        };

        assert_relative_eq!(actual.value, expected.value, epsilon = 1e-4);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_calmar_ratio_scale_custom_intervals() {
        let two_hour = CalmarRatio {
            value: 0.05,
            interval: TimeDelta::hours(2),
        };

        let actual = two_hour.scale(TimeDelta::hours(8));

        let expected = CalmarRatio {
            value: 0.05 * (4.0_f64).sqrt(), // 4 = 8 hours / 2 hours
            interval: TimeDelta::hours(8),
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_calmar_ratio_extreme_values() {
        // Test with very small values
        let small = CalmarRatio::calculate(1e-10, 2e-10, 1e-10, Daily);
        assert_relative_eq!(small.value, 1.0, epsilon = 1e-4);

        // Test with very large values
        let large = CalmarRatio::calculate(1e10, 2e10, 1e10, Daily);
        assert_relative_eq!(large.value, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn test_calmar_ratio_absolute_drawdown() {
        // Test that negative drawdown values are handled correctly (absolute value is used)
        let risk_free_return = 0.001;
        let mean_return = 0.002;
        let negative_drawdown = -0.015; // Should be treated same as positive 0.015
        let time_period = Daily;

        let actual = CalmarRatio::calculate(
            risk_free_return,
            mean_return,
            negative_drawdown,
            time_period,
        );

        let expected = CalmarRatio {
            value: (0.002 - 0.001) / 0.015,
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }
}
