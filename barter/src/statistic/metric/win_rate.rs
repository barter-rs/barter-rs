use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct WinRate {
    pub value: Decimal,
}

impl WinRate {
    pub fn calculate(wins: Decimal, total: Decimal) -> Option<Self> {
        if total == Decimal::ZERO {
            None
        } else {
            let value = wins.abs().checked_div(total.abs())?;
            Some(Self { value })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_win_rate_calculate() {
        // no trades
        assert_eq!(WinRate::calculate(Decimal::ZERO, Decimal::ZERO), None);

        // all winning trades
        assert_eq!(
            WinRate::calculate(Decimal::TEN, Decimal::TEN)
                .unwrap()
                .value,
            Decimal::ONE
        );

        // no winning trades
        assert_eq!(
            WinRate::calculate(Decimal::ZERO, Decimal::TEN)
                .unwrap()
                .value,
            Decimal::ZERO
        );

        // mixed winning and losing trades
        assert_eq!(
            WinRate::calculate(dec!(6), Decimal::TEN).unwrap().value,
            dec!(0.6)
        );
    }
}
