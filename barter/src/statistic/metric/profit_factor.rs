use rust_decimal::Decimal;
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
    pub value: Decimal,
}

impl ProfitFactor {
    pub fn calculate(profits_gross_abs: Decimal, losses_gross_abs: Decimal) -> Self {
        Self {
            value: if profits_gross_abs.is_zero() && losses_gross_abs.is_zero() {
                Decimal::ONE
            } else if losses_gross_abs.is_zero() {
                Decimal::MAX
            } else if profits_gross_abs.is_zero() {
                Decimal::MIN
            } else {
                profits_gross_abs
                    .abs()
                    .checked_div(losses_gross_abs.abs())
                    .unwrap()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    #[test]
    fn test_profit_factor_calculate() {
        // both profits & losses are very small
        assert_eq!(
            ProfitFactor::calculate(
                Decimal::from_scientific("1e-20").unwrap(),
                Decimal::from_scientific("1e-20").unwrap()
            )
            .value,
            Decimal::ONE
        );

        // both profits & losses are very large
        assert_eq!(
            ProfitFactor::calculate(Decimal::MAX / dec!(2), Decimal::MAX / dec!(2)).value,
            Decimal::ONE
        );

        // both profits & losses are zero
        assert_eq!(
            ProfitFactor::calculate(dec!(0.0), dec!(0.0)).value,
            Decimal::ONE
        );

        // profits are zero
        assert_eq!(
            ProfitFactor::calculate(dec!(0.0), dec!(1.0)).value,
            Decimal::MIN
        );

        // losses are zero
        assert_eq!(
            ProfitFactor::calculate(dec!(1.0), dec!(0.0)).value,
            Decimal::MAX
        );

        // both profits & losses are non-zero
        assert_eq!(
            ProfitFactor::calculate(dec!(10.0), dec!(5.0)).value,
            dec!(2.0)
        );

        // both profits & losses are non-zero, but input losses are not abs
        assert_eq!(
            ProfitFactor::calculate(dec!(10.0), dec!(-5.0)).value,
            dec!(2.0)
        );

        // test with precise decimal values
        assert_eq!(
            ProfitFactor::calculate(dec!(10.5555), dec!(5.2345)).value,
            Decimal::from_str("2.016524978507975928933040405").unwrap()
        );
    }
}
