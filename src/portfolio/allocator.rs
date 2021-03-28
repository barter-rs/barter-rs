use crate::portfolio::order::OrderEvent;
use crate::portfolio::position::Position;
use crate::portfolio::error::PortfolioError;
use crate::strategy::signal::{SignalStrength, Decision};

use serde::Deserialize;

/// Allocates an appropriate OrderEvent quantity.
pub trait OrderAllocator {
    /// Returns an OrderEvent with a calculated order quantity based on the input order,
    /// SignalStrength and potential existing Position.
    fn allocate_order(&self, order: OrderEvent, position: Option<&Position>,
                      signal_strength: SignalStrength) -> Result<OrderEvent, PortfolioError>;
}

/// Default allocation manager that implements OrderAllocator. Order size is calculated by using the
/// default_order_value, symbol close value, and SignalStrength.
#[derive(Debug, Deserialize)]
pub struct DefaultAllocator {
    pub default_order_value: f64,
}

impl OrderAllocator for DefaultAllocator {
    fn allocate_order(&self, mut order: OrderEvent, position: Option<&Position>,
                      signal_strength: SignalStrength) -> Result<OrderEvent, PortfolioError> {
        let default_order_size = (self.default_order_value / order.close).floor();

        match order.decision {
            // Entry
            Decision::Long => order.quantity = default_order_size * signal_strength as f64,

            // Entry
            Decision::Short => order.quantity = -default_order_size * signal_strength as f64,

            // Exit
            _ => order.quantity = 0.0 - position.unwrap().quantity,
        }
        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_allocate_order_to_exit_open_long_position() {
        let allocator = DefaultAllocator {default_order_value: 1000.0};

        let mut input_order = OrderEvent::default();
        input_order.decision = Decision::CloseLong;

        let mut input_position = Position::default();
        input_position.quantity = 100.0;

        let input_signal_strength = 0.0;

        let actual_result = allocator.allocate_order(
            input_order, Some(&input_position), input_signal_strength
        ).unwrap().quantity;

        let expected_result = 0.0 - input_position.quantity;

        assert_eq!(actual_result, expected_result)
    }

    #[test]
    fn should_allocate_order_to_exit_open_short_position() {
        let allocator = DefaultAllocator {default_order_value: 1000.0};

        let mut input_order = OrderEvent::default();
        input_order.decision = Decision::CloseShort;

        let mut input_position = Position::default();
        input_position.quantity = -100.0;

        let input_signal_strength = 0.0;

        let actual_result = allocator.allocate_order(
            input_order, Some(&input_position), input_signal_strength
        ).unwrap().quantity;

        let expected_result = 0.0 - input_position.quantity;

        assert_eq!(actual_result, expected_result)
    }

    #[test]
    fn should_allocate_order_to_enter_long_position_with_correct_quantity() {
        let default_order_value = 1000.0;
        let allocator = DefaultAllocator {default_order_value};

        let order_close = 10.0;
        let mut input_order = OrderEvent::default();
        input_order.close = order_close;
        input_order.decision = Decision::Long;

        let input_signal_strength = 1.0;

        let actual_result = allocator.allocate_order(
            input_order, None, input_signal_strength
        ).unwrap().quantity;

        let expected_result = (default_order_value / order_close) * input_signal_strength as f64;

        assert_eq!(actual_result, expected_result)
    }

    #[test]
    fn should_allocate_order_to_enter_close_position_with_correct_quantity() {
        let default_order_value = 1000.0;
        let allocator = DefaultAllocator {default_order_value};

        let order_close = 10.0;
        let mut input_order = OrderEvent::default();
        input_order.close = order_close;
        input_order.decision = Decision::Short;

        let input_signal_strength = 1.0;

        let actual_result = allocator.allocate_order(
            input_order, None, input_signal_strength
        ).unwrap().quantity;

        let expected_result = -(default_order_value / order_close) * input_signal_strength as f64;

        assert_eq!(actual_result, expected_result)
    }
}