use crate::statistic::time::TimeInterval;
use rust_decimal::{Decimal, MathematicalOps};
use serde::{Deserialize, Serialize};

/// Represents a Sharpe Ratio value over a specific [`TimeInterval`].
///
/// Sharpe Ratio measures the risk-adjusted return of an investment by comparing
/// its excess returns (over risk-free rate) to its standard deviation.
///
/// See docs: <https://www.investopedia.com/articles/07/sharpe_ratio.asp>
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct SharpeRatio<Interval> {
    pub value: Decimal,
    pub interval: Interval,
}

impl<Interval> SharpeRatio<Interval>
where
    Interval: TimeInterval,
{
    /// Calculate the [`SharpeRatio`] over the provided [`TimeInterval`].
    pub fn calculate(
        risk_free_return: Decimal,
        mean_return: Decimal,
        std_dev_returns: Decimal,
        returns_period: Interval,
    ) -> Self {
        if std_dev_returns.is_zero() {
            Self {
                value: Decimal::MAX,
                interval: returns_period,
            }
        } else {
            let excess_returns = mean_return - risk_free_return;
            let ratio = excess_returns.checked_div(std_dev_returns).unwrap();
            Self {
                value: ratio,
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
        let target_secs = Decimal::from(target.interval().num_seconds());
        let current_secs = Decimal::from(self.interval.interval().num_seconds());

        let scale = target_secs
            .abs()
            .checked_div(current_secs.abs())
            .unwrap_or(Decimal::MAX)
            .sqrt()
            .expect("ensured seconds are Positive");

        SharpeRatio {
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

    #[test]
    fn test_sharpe_ratio_with_zero_std_dev() {
        let risk_free_return = dec!(0.001);
        let mean_return = dec!(0.002);
        let std_dev_returns = dec!(0.0);
        let time_period = TimeDelta::hours(2);

        let result =
            SharpeRatio::calculate(risk_free_return, mean_return, std_dev_returns, time_period);
        assert_eq!(result.value, Decimal::MAX);
    }

    #[test]
    fn test_sharpe_ratio_calculate_with_custom_interval() {
        // Define custom interval returns statistics
        let risk_free_return = dec!(0.0015); // 0.15%
        let mean_return = dec!(0.0025); // 0.25%
        let std_dev_returns = dec!(0.02); // 2%
        let time_period = TimeDelta::hours(2);

        let actual =
            SharpeRatio::calculate(risk_free_return, mean_return, std_dev_returns, time_period);

        let expected = SharpeRatio {
            value: dec!(0.05),
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sharpe_ratio_calculate_with_daily_interval() {
        // Define daily returns statistics
        let risk_free_return = dec!(0.0015); // 0.15%
        let mean_return = dec!(0.0025); // 0.25%
        let std_dev_returns = dec!(0.02); // 2%
        let time_period = Daily;

        let actual =
            SharpeRatio::calculate(risk_free_return, mean_return, std_dev_returns, time_period);

        let expected = SharpeRatio {
            value: dec!(0.05),
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_sharpe_ratio_scale_from_daily_to_annual_252() {
        let input = SharpeRatio {
            value: dec!(0.05),
            interval: Daily,
        };

        let actual = input.scale(Annual252);

        let expected = SharpeRatio {
            value: dec!(0.7937253933193771771504847261),
            interval: Annual252,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }
}
