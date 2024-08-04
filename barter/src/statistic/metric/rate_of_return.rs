use crate::statistic::time::TimeInterval;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Represents a Rate of Return value over a specific [`TimeInterval`].
///
/// Rate of Return measures the percentage change in value over a time period.
/// Unlike risk-adjusted metrics, returns scale linearly with time.
///
/// See docs: <https://www.investopedia.com/terms/r/rateofreturn.asp>
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct RateOfReturn<Interval> {
    pub value: Decimal,
    pub interval: Interval,
}

impl<Interval> RateOfReturn<Interval>
where
    Interval: TimeInterval,
{
    /// Calculate the [`RateOfReturn`] over the provided [`TimeInterval`].
    pub fn calculate(mean_return: Decimal, returns_period: Interval) -> Self {
        Self {
            value: mean_return,
            interval: returns_period,
        }
    }

    /// Scale the [`RateOfReturn`] from the current [`TimeInterval`] to the provided
    /// [`TimeInterval`].
    ///
    /// Unlike risk metrics which use square root scaling, [`RateOfReturn`] scales linearly
    /// with time.
    ///
    /// For example, a 1% daily return scales to approximately 252% annual return (not √252%).
    ///
    /// This assumes simple interest rather than compound interest.
    pub fn scale<TargetInterval>(self, target: TargetInterval) -> RateOfReturn<TargetInterval>
    where
        TargetInterval: TimeInterval,
    {
        // Determine scale factor: linear scaling of Self Intervals in TargetIntervals
        let target_secs = Decimal::from(target.interval().num_seconds());
        let current_secs = Decimal::from(self.interval.interval().num_seconds());

        let scale = target_secs
            .abs()
            .checked_div(current_secs.abs())
            .unwrap_or(Decimal::MAX);

        RateOfReturn {
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
    fn test_rate_of_return_normal_case() {
        let mean_return = dec!(0.0025); // 0.25%
        let time_period = Daily;

        let actual = RateOfReturn::calculate(mean_return, time_period);

        let expected = RateOfReturn {
            value: dec!(0.0025),
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_rate_of_return_zero() {
        let mean_return = dec!(0.0);
        let time_period = Daily;

        let actual = RateOfReturn::calculate(mean_return, time_period);

        assert_eq!(actual.value, dec!(0.0));
        assert_eq!(actual.interval, time_period);
    }

    #[test]
    fn test_rate_of_return_negative() {
        let mean_return = dec!(-0.0025); // -0.25%
        let time_period = Daily;

        let actual = RateOfReturn::calculate(mean_return, time_period);

        let expected = RateOfReturn {
            value: dec!(-0.0025),
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_rate_of_return_custom_interval() {
        let mean_return = dec!(0.0025); // 0.25%
        let time_period = TimeDelta::hours(4);

        let actual = RateOfReturn::calculate(mean_return, time_period);

        let expected = RateOfReturn {
            value: dec!(0.0025),
            interval: time_period,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_rate_of_return_scale_daily_to_annual() {
        // For returns, we use linear scaling (multiply by 252) not square root scaling
        let daily = RateOfReturn {
            value: dec!(0.01), // 1% daily return
            interval: Daily,
        };

        let actual = daily.scale(Annual252);

        let expected = RateOfReturn {
            value: dec!(2.52), // Should be 252% annual return
            interval: Annual252,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_rate_of_return_scale_custom_intervals() {
        // Test scaling from 2 hours to 8 hours (linear scaling factor of 4)
        let two_hour = RateOfReturn {
            value: dec!(0.01), // 1% per 2 hours
            interval: TimeDelta::hours(2),
        };

        let actual = two_hour.scale(TimeDelta::hours(8));

        let expected = RateOfReturn {
            value: dec!(0.04), // Should be 4% per 8 hours
            interval: TimeDelta::hours(8),
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_rate_of_return_scale_zero() {
        // Zero returns should remain zero when scaled
        let daily = RateOfReturn {
            value: dec!(0.0),
            interval: Daily,
        };

        let actual = daily.scale(Annual252);

        assert_eq!(actual.value, dec!(0.0));
        assert_eq!(actual.interval, Annual252);
    }

    #[test]
    fn test_rate_of_return_scale_negative() {
        // Negative returns should scale linearly while maintaining sign
        let daily = RateOfReturn {
            value: dec!(-0.01), // -1% daily return
            interval: Daily,
        };

        let actual = daily.scale(Annual252);

        let expected = RateOfReturn {
            value: dec!(-2.52), // Should be -252% annual return
            interval: Annual252,
        };

        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.interval, expected.interval);
    }

    #[test]
    fn test_rate_of_return_extreme_values() {
        // Test with very small values
        let small = RateOfReturn::calculate(dec!(1e-10), Daily);
        let small_annual = small.scale(Annual252);
        assert_eq!(small_annual.value, dec!(252e-10));

        // Test with very large values
        let large = RateOfReturn::calculate(dec!(1e10), Daily);
        let large_annual = large.scale(Annual252);
        assert_eq!(large_annual.value, dec!(252e10));
    }
}
