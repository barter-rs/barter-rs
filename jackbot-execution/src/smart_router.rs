use rust_decimal::Decimal;
use std::collections::HashMap;
use jackbot_instrument::exchange::ExchangeId;

/// Simple smart routing state with exposure tracking.
#[derive(Debug)]
pub struct SmartRouter {
    max_exposure: Decimal,
    current_exposure: Decimal,
}

impl SmartRouter {
    /// Create a new router with the specified maximum exposure.
    pub fn new(max_exposure: Decimal) -> Self {
        Self { max_exposure, current_exposure: Decimal::ZERO }
    }

    /// Returns true if a trade of `quantity` would exceed exposure limits.
    pub fn can_execute(&self, quantity: Decimal) -> bool {
        self.current_exposure + quantity <= self.max_exposure
    }

    /// Record a new executed quantity, increasing exposure. Returns Err if the
    /// exposure would exceed the configured limit.
    pub fn record_execution(&mut self, quantity: Decimal) -> Result<(), Decimal> {
        if self.can_execute(quantity) {
            self.current_exposure += quantity;
            Ok(())
        } else {
            Err(self.current_exposure + quantity - self.max_exposure)
        }
    }

    /// Reduce exposure after a position is closed or filled.
    pub fn reduce_exposure(&mut self, quantity: Decimal) {
        self.current_exposure -= quantity;
        if self.current_exposure < Decimal::ZERO {
            self.current_exposure = Decimal::ZERO;
        }
    }

    /// Current exposure value.
    pub fn exposure(&self) -> Decimal {
        self.current_exposure
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn router_enforces_limit() {
        let mut router = SmartRouter::new(dec!(5));
        assert!(router.record_execution(dec!(3)).is_ok());
        assert!(router.record_execution(dec!(2)).is_ok());
        assert!(router.record_execution(dec!(1)).is_err());
        assert_eq!(router.exposure(), dec!(5));
        router.reduce_exposure(dec!(2));
        assert!(router.record_execution(dec!(1)).is_ok());
    }
}
