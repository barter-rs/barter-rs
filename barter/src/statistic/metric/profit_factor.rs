use rust_decimal::prelude::Zero;
use serde::{Deserialize, Serialize};

/// ProfitFactor is a performance metric that divides the absolute value of gross profits
/// by the absolute value of gross losses. A profit factor greater than 1 indicates a profitable
/// strategy.
///
/// Special cases:
/// - Returns 1.0 when both profits and losses are zero (neutral performance)
/// - Returns INFINITY when there are profits but no losses (perfect performance)
/// - Returns NEG_INFINITY when there are losses but no profits (worst performance)
///
/// See docs: <https://www.investopedia.com/articles/fundamental-analysis/10/strategy-performance-reports.asp#toc-profit-factor>
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct ProfitFactor {
    pub value: f64,
}

impl ProfitFactor {
    pub fn calculate(profits_gross_abs: f64, losses_gross_abs: f64) -> Self {
        if profits_gross_abs.is_nan() || losses_gross_abs.is_nan() {
            return Self { value: f64::NAN };
        }

        Self {
            value: if profits_gross_abs.is_zero() && losses_gross_abs.is_zero() {
                1.0
            } else if losses_gross_abs.is_zero() {
                f64::INFINITY
            } else if profits_gross_abs.is_zero() {
                f64::NEG_INFINITY
            } else {
                profits_gross_abs.abs() / losses_gross_abs.abs()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profit_factor_calculate() {
        // profits are NAN
        assert!(ProfitFactor::calculate(f64::NAN, 5.0).value.is_nan());

        // losses are NAN
        assert!(ProfitFactor::calculate(5.0, f64::NAN).value.is_nan());

        // both profits & losses are very small
        assert_eq!(
            ProfitFactor::calculate(f64::EPSILON, f64::EPSILON).value,
            1.0
        );

        // both profits & losses are very small
        assert_eq!(
            ProfitFactor::calculate(f64::MAX / 2.0, f64::MAX / 2.0).value,
            1.0
        );

        // both profits & losses are zero
        assert_eq!(ProfitFactor::calculate(0.0, 0.0).value, 1.0);

        // profits are zero
        assert_eq!(ProfitFactor::calculate(0.0, 1.0).value, f64::NEG_INFINITY);

        // losses are zero
        assert_eq!(ProfitFactor::calculate(1.0, 0.0).value, f64::INFINITY);

        // both profits & losses are non-zero
        assert_eq!(ProfitFactor::calculate(10.0, 5.0).value, 2.0);

        // both profits & losses are non-zero, but input losses are not abs
        assert_eq!(ProfitFactor::calculate(10.0, -5.0).value, 2.0);
    }
}
