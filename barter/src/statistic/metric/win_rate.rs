use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Represents a win rate ratio between 0 and 1, calculated as `wins/total`.
///
/// The win rate is calculated as the absolute ratio of winning trades to total trades.
///
/// Returns None if there are no trades (total = 0) or if the division operation overflows.
///
/// See docs: <https://www.investopedia.com/terms/w/win-loss-ratio.asp>
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct WinRate {
    pub value: Decimal,
}

impl WinRate {
    /// Calculate the [`WinRate`] given the provided number of wins and total positions.
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
