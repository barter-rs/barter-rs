use crate::statistic::time::TimeInterval;
use rust_decimal::{Decimal, MathematicalOps};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Represents a Calmar Ratio value over a specific [`TimeInterval`].
///
/// The Calmar Ratio is a risk-adjusted return measure that divides the excess return
/// (over risk-free rate) by the Maximum Drawdown risk. It's similar to the Sharpe and Sortino
/// ratios, but uses Maximum Drawdown as the risk measure instead of standard deviation.
///
/// See docs: <https://corporatefinanceinstitute.com/resources/career-map/sell-side/capital-markets/calmar-ratio/>
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct CalmarRatio<Interval> {
    pub value: Decimal,
    pub interval: Interval,
}

impl<Interval> CalmarRatio<Interval>
where
    Interval: TimeInterval,
{
    /// Calculate the [`CalmarRatio`] over the provided [`TimeInterval`].
    pub fn calculate(
        risk_free_return: Decimal,
        mean_return: Decimal,
        max_drawdown: Decimal,
        returns_period: Interval,
    ) -> Self {
        if max_drawdown.is_zero() {
            Self {
                value: match mean_return.cmp(&risk_free_return) {
                    // Special case: +ve excess returns with no drawdown risk (very good)
                    Ordering::Greater => Decimal::MAX,
                    // Special case: -ve excess returns with no drawdown risk (very bad)
                    Ordering::Less => Decimal::MIN,
                    // Special case: no excess returns with no drawdown risk (neutral)
                    Ordering::Equal => Decimal::ZERO,
                },
                interval: returns_period,
            }
        } else {
            let excess_returns = mean_return - risk_free_return;
            let ratio = excess_returns.checked_div(max_drawdown.abs()).unwrap();
            Self {
                value: ratio,
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
        let target_secs = Decimal::from(target.interval().num_seconds());
        let current_secs = Decimal::from(self.interval.interval().num_seconds());

        let scale = target_secs
            .abs()
            .checked_div(current_secs.abs())
            .unwrap_or(Decimal::MAX)
            .sqrt()
            .expect("ensured seconds are Positive");

        CalmarRatio {
            value: self.value.checked_mul(scale).unwrap_or(Decimal::MAX),
            interval: target,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistic::time::{Annual252, Daily};
    use chrono::TimeDelta;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    #[test]
    fn test_calmar_ratio_normal_case() {
        let risk_free_return = dec!(0.0015); // 0.15%
        let mean_return = dec!(0.0025); // 0.25%
        let max_drawdown = dec!(0.02); // 2%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        let expected = CalmarRatio {
            value: dec!(0.05), // (0.0025 - 0.0015) / 0.02
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_calmar_ratio_zero_drawdown_positive_excess() {
        let risk_free_return = dec!(0.001); // 0.1%
        let mean_return = dec!(0.002); // 0.2%
        let max_drawdown = dec!(0.0); // 0%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        assert_eq!(actual.value, Decimal::MAX);
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_calmar_ratio_zero_drawdown_negative_excess_returns() {
        let risk_free_return = dec!(0.002); // 0.2%
        let mean_return = dec!(0.001); // 0.1%
        let max_drawdown = dec!(0.0); // 0%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        assert_eq!(actual.value, Decimal::MIN);
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_calmar_ratio_zero_drawdown_negative_excess_via_negative_returns() {
        let risk_free_return = dec!(0.002); // 0.2%
        let mean_return = dec!(-0.001); // -0.1%
        let max_drawdown = dec!(0.0); // 0%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        assert_eq!(actual.value, Decimal::MIN);
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_calmar_ratio_zero_drawdown_no_excess_returns() {
        let risk_free_return = dec!(0.001); // 0.1%
        let mean_return = dec!(0.001); // 0.1%
        let max_drawdown = dec!(0.0); // 0%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        assert_eq!(actual.value, dec!(0.0));
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_calmar_ratio_negative_returns() {
        let risk_free_return = dec!(0.001); // 0.1%
        let mean_return = dec!(-0.002); // -0.2%
        let max_drawdown = dec!(0.015); // 1.5%
        let time_period = Daily;

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        let expected = CalmarRatio {
            value: Decimal::from_str("-0.2").unwrap(), // (-0.002 - 0.001) / 0.015
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_calmar_ratio_custom_interval() {
        let risk_free_return = dec!(0.0015); // 0.15%
        let mean_return = dec!(0.0025); // 0.25%
        let max_drawdown = dec!(0.02); // 2%
        let time_period = TimeDelta::hours(4);

        let actual =
            CalmarRatio::calculate(risk_free_return, mean_return, max_drawdown, time_period);

        let expected = CalmarRatio {
            value: dec!(0.05),
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_calmar_ratio_scale_daily_to_annual() {
        let daily = CalmarRatio {
            value: dec!(0.05),
            interval: Daily,
        };

        let actual = daily.scale(Annual252);

        // 0.05 * sqrt(252) ≈ 0.7937
        let expected = CalmarRatio {
            value: Decimal::from_str("0.7937").unwrap(),
            interval: Annual252,
        };

        let diff = (actual.value - expected.value).abs();
        assert!(diff <= Decimal::from_str("0.0001").unwrap());
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_calmar_ratio_scale_custom_intervals() {
        let two_hour = CalmarRatio {
            value: dec!(0.05),
            interval: TimeDelta::hours(2),
        };

        let actual = two_hour.scale(TimeDelta::hours(8));

        // 0.05 * sqrt(4) = 0.05 * 2 = 0.1
        let expected = CalmarRatio {
            value: dec!(0.1),
            interval: TimeDelta::hours(8),
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_calmar_ratio_extreme_values() {
        // Test with very small values
        let small = CalmarRatio::calculate(
            Decimal::from_scientific("1e-10").unwrap(),
            Decimal::from_scientific("2e-10").unwrap(),
            Decimal::from_scientific("1e-10").unwrap(),
            Daily,
        );

        assert_eq!(small.value, dec!(1.0));

        // Test with very large values
        let large = CalmarRatio::calculate(
            Decimal::from_scientific("1e10").unwrap(),
            Decimal::from_scientific("2e10").unwrap(),
            Decimal::from_scientific("1e10").unwrap(),
            Daily,
        );

        assert_eq!(large.value, dec!(1.0));
    }

    #[test]
    fn test_calmar_ratio_absolute_drawdown() {
        // Test that negative drawdown values are handled correctly (absolute value is used)
        let risk_free_return = dec!(0.001);
        let mean_return = dec!(0.002);
        let negative_drawdown = dec!(-0.015); // Should be treated same as positive 0.015
        let time_period = Daily;

        let actual = CalmarRatio::calculate(
            risk_free_return,
            mean_return,
            negative_drawdown,
            time_period,
        );

        let expected = CalmarRatio {
            value: Decimal::from_str("0.0666666666666666666666666667").unwrap(), // (0.002 - 0.001) / 0.015
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }
}
