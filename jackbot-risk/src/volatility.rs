use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Utility for scaling position sizes and risk limits based on volatility.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Constructor)]
pub struct VolatilityScaler {
    /// Baseline volatility used when the scale factor is 1.0.
    pub base_volatility: Decimal,
    /// Minimum allowed scaling factor.
    pub min_scale: Decimal,
    /// Maximum allowed scaling factor.
    pub max_scale: Decimal,
}

impl VolatilityScaler {
    /// Calculate a scaling factor for the provided `volatility`.
    /// Values are clamped between `min_scale` and `max_scale`.
    pub fn scale(&self, volatility: Decimal) -> Decimal {
        if volatility <= Decimal::ZERO {
            return self.max_scale;
        }
        let mut factor = self.base_volatility / volatility;
        if factor < self.min_scale {
            factor = self.min_scale;
        } else if factor > self.max_scale {
            factor = self.max_scale;
        }
        factor
    }

    /// Adjust a base position size using the calculated scaling factor.
    pub fn adjust_position(&self, base_size: Decimal, volatility: Decimal) -> Decimal {
        base_size * self.scale(volatility)
    }

    /// Adjust a risk limit using the calculated scaling factor.
    pub fn adjust_risk(&self, base_limit: Decimal, volatility: Decimal) -> Decimal {
        base_limit * self.scale(volatility)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_scale_bounds() {
        let scaler = VolatilityScaler::new(dec!(0.02), dec!(0.5), dec!(2));
        assert_eq!(scaler.scale(dec!(0.04)), dec!(0.5));
        assert_eq!(scaler.scale(dec!(0.01)), dec!(2));
        assert_eq!(scaler.scale(dec!(0)), dec!(2));
    }

    #[test]
    fn test_adjust_position() {
        let scaler = VolatilityScaler::new(dec!(0.02), dec!(0.5), dec!(2));
        let adj = scaler.adjust_position(dec!(10), dec!(0.04));
        assert_eq!(adj, dec!(5));
    }
}
