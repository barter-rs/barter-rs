use crate::statistic::time::TimeInterval;
use rust_decimal::{Decimal, MathematicalOps};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Represents a Sortino Ratio value over a specific [`TimeInterval`].
///
/// Similar to the Sharpe Ratio, but only considers downside volatility (standard deviation of
/// negative returns) rather than total volatility. This makes it a better metric for portfolios
/// with non-normal return distributions.
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct SortinoRatio<Interval> {
    pub value: Decimal,
    pub interval: Interval,
}

impl<Interval> SortinoRatio<Interval>
where
    Interval: TimeInterval,
{
    /// Calculate the [`SortinoRatio`] over the provided [`TimeInterval`].
    pub fn calculate(
        risk_free_return: Decimal,
        mean_return: Decimal,
        std_dev_loss_returns: Decimal,
        returns_period: Interval,
    ) -> Self {
        if std_dev_loss_returns.is_zero() {
            Self {
                value: match mean_return.cmp(&risk_free_return) {
                    // Special case: +ve excess returns with no downside risk (very good)
                    Ordering::Greater => Decimal::MAX,
                    // Special case: -ve excess returns with no downside risk (very bad)
                    Ordering::Less => Decimal::MIN,
                    // Special case: no excess returns with no downside risk (neutral)
                    Ordering::Equal => Decimal::ZERO,
                },
                interval: returns_period,
            }
        } else {
            let excess_returns = mean_return - risk_free_return;
            let ratio = excess_returns.checked_div(std_dev_loss_returns).unwrap();
            Self {
                value: ratio,
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
        let target_secs = Decimal::from(target.interval().num_seconds());
        let current_secs = Decimal::from(self.interval.interval().num_seconds());

        let scale = target_secs
            .abs()
            .checked_div(current_secs.abs())
            .unwrap_or(Decimal::MAX)
            .sqrt()
            .expect("ensured seconds are Positive");

        SortinoRatio {
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
    fn test_sortino_ratio_normal_case() {
        // Define test case with reasonable values
        let risk_free_return = dec!(0.0015); // 0.15%
        let mean_return = dec!(0.0025); // 0.25%
        let std_dev_loss_returns = dec!(0.02); // 2%
        let time_period = Daily;

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        let expected = SortinoRatio {
            value: dec!(0.05), // (0.0025 - 0.0015) / 0.02
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_sortino_ratio_zero_downside_dev_positive_excess() {
        // Test case: positive excess returns with no downside risk
        let risk_free_return = dec!(0.001); // 0.1%
        let mean_return = dec!(0.002); // 0.2%
        let std_dev_loss_returns = dec!(0.0);
        let time_period = Daily;

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        assert_eq!(actual.value, Decimal::MAX);
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_sortino_ratio_zero_downside_dev_negative_excess() {
        // Test case: negative excess returns with no downside risk
        let risk_free_return = dec!(0.002); // 0.2%
        let mean_return = dec!(0.001); // 0.1%
        let std_dev_loss_returns = dec!(0.0);
        let time_period = Daily;

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        assert_eq!(actual.value, Decimal::MIN);
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_sortino_ratio_zero_downside_dev_no_excess() {
        // Test case: no excess returns with no downside risk
        let risk_free_return = dec!(0.001); // 0.1%
        let mean_return = dec!(0.001); // 0.1%
        let std_dev_loss_returns = dec!(0.0);
        let time_period = Daily;

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        assert_eq!(actual.value, dec!(0.0));
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_sortino_ratio_negative_returns() {
        // Test case: negative mean returns
        let risk_free_return = dec!(0.001); // 0.1%
        let mean_return = dec!(-0.002); // -0.2%
        let std_dev_loss_returns = dec!(0.015); // 1.5%
        let time_period = Daily;

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        let expected = SortinoRatio {
            value: dec!(-0.2), // (-0.002 - 0.001) / 0.015
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sortino_ratio_custom_interval() {
        // Test case with custom time interval
        let risk_free_return = dec!(0.0015); // 0.15%
        let mean_return = dec!(0.0025); // 0.25%
        let std_dev_loss_returns = dec!(0.02); // 2%
        let time_period = TimeDelta::hours(4);

        let actual = SortinoRatio::calculate(
            risk_free_return,
            mean_return,
            std_dev_loss_returns,
            time_period,
        );

        let expected = SortinoRatio {
            value: dec!(0.05),
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sortino_ratio_scale_daily_to_annual() {
        // Test scaling from daily to annual
        let daily = SortinoRatio {
            value: dec!(0.05),
            interval: Daily,
        };

        let actual = daily.scale(Annual252);

        // 0.05 * √252 ≈ 0.7937
        let expected = SortinoRatio {
            value: Decimal::from_str("0.7937").unwrap(),
            interval: Annual252,
        };

        let diff = (actual.value - expected.value).abs();
        assert!(diff <= Decimal::from_str("0.0001").unwrap());
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sortino_ratio_scale_custom_intervals() {
        // Test scaling between custom intervals
        let two_hour = SortinoRatio {
            value: dec!(0.05),
            interval: TimeDelta::hours(2),
        };

        let actual = two_hour.scale(TimeDelta::hours(8));

        // 0.05 * √4 = 0.1
        let expected = SortinoRatio {
            value: dec!(0.1),
            interval: TimeDelta::hours(8),
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sortino_ratio_extreme_values() {
        // Test with very small values
        let small = SortinoRatio::calculate(
            Decimal::from_scientific("1e-10").unwrap(),
            Decimal::from_scientific("2e-10").unwrap(),
            Decimal::from_scientific("1e-10").unwrap(),
            Daily,
        );

        let diff = (small.value - dec!(1.0)).abs();
        assert!(diff <= Decimal::from_str("0.0001").unwrap());

        // Test with very large values
        let large = SortinoRatio::calculate(
            Decimal::from_scientific("1e10").unwrap(),
            Decimal::from_scientific("2e10").unwrap(),
            Decimal::from_scientific("1e10").unwrap(),
            Daily,
        );

        let diff = (large.value - dec!(1.0)).abs();
        assert!(diff <= Decimal::from_str("0.0001").unwrap());
    }
}
