use crate::{
    portfolio::{position::Position, OrderEvent},
    strategy::{Decision, SignalStrength},
};
use serde::{Deserialize, Serialize};

/// Allocates an appropriate [`OrderEvent`] quantity.
pub trait OrderAllocator {
    /// Returns an [`OrderEvent`] with a calculated order quantity based on the input order,
    /// [`SignalStrength`] and potential existing [`Position`].
    fn allocate_order(
        &self,
        order: &mut OrderEvent,
        position: Option<&Position>,
        signal_strength: SignalStrength,
    );
}

/// Default allocation manager that implements [`OrderAllocator`]. Order size is calculated by
/// using the default_order_value, symbol close value, and [`SignalStrength`].
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default, Deserialize, Serialize)]
pub struct DefaultAllocator {
    pub default_order_value: f64,
}

impl OrderAllocator for DefaultAllocator {
    fn allocate_order(
        &self,
        order: &mut OrderEvent,
        position: Option<&Position>,
        signal_strength: SignalStrength,
    ) {
        // Calculate exact order_size, then round it to a more appropriate decimal place
        let default_order_size = self.default_order_value / order.market_meta.close;
        let default_order_size = (default_order_size * 10000.0).floor() / 10000.0;

        match order.decision {
            // Entry
            Decision::Long => order.quantity = default_order_size * signal_strength.0,

            // Entry
            Decision::Short => order.quantity = -default_order_size * signal_strength.0,

            // Exit
            _ => order.quantity = 0.0 - position.as_ref().unwrap().quantity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{order_event, position};

    #[test]
    fn should_allocate_order_to_exit_open_long_position() {
        let allocator = DefaultAllocator {
            default_order_value: 1000.0,
        };

        let mut input_order = order_event();
        input_order.decision = Decision::CloseLong;

        let mut input_position = position();
        input_position.quantity = 100.0;

        let input_signal_strength = SignalStrength(0.0);

        allocator.allocate_order(
            &mut input_order,
            Some(&input_position),
            input_signal_strength,
        );

        let actual_result = input_order.quantity;
        let expected_result = 0.0 - input_position.quantity;

        assert_eq!(actual_result, expected_result)
    }

    #[test]
    fn should_allocate_order_to_exit_open_short_position() {
        let allocator = DefaultAllocator {
            default_order_value: 1000.0,
        };

        let mut input_order = order_event();
        input_order.decision = Decision::CloseShort;

        let mut input_position = position();
        input_position.quantity = -100.0;

        let input_signal_strength = SignalStrength(0.0);

        allocator.allocate_order(
            &mut input_order,
            Some(&input_position),
            input_signal_strength,
        );

        let actual_result = input_order.quantity;
        let expected_result = 0.0 - input_position.quantity;

        assert_eq!(actual_result, expected_result)
    }

    #[test]
    fn should_allocate_order_to_enter_long_position_with_correct_quantity() {
        let default_order_value = 1000.0;
        let allocator = DefaultAllocator {
            default_order_value,
        };

        let order_close = 10.0;
        let mut input_order = order_event();
        input_order.market_meta.close = order_close;
        input_order.decision = Decision::Long;

        let input_signal_strength = SignalStrength(1.0);

        allocator.allocate_order(&mut input_order, None, input_signal_strength);

        let actual_result = input_order.quantity;
        let expected_result = (default_order_value / order_close) * input_signal_strength.0 as f64;

        assert_eq!(actual_result, expected_result)
    }

    #[test]
    fn should_allocate_order_to_enter_long_position_with_non_zero_quantity() {
        let default_order_value = 200.0;
        let allocator = DefaultAllocator {
            default_order_value,
        };

        let order_close = 226.753403;
        let mut input_order = order_event();
        input_order.market_meta.close = order_close;
        input_order.decision = Decision::Long;

        let input_signal_strength = SignalStrength(1.0);

        allocator.allocate_order(&mut input_order, None, input_signal_strength);

        let actual_result = input_order.quantity;
        let expected_order_size = ((default_order_value / order_close) * 10000.0).floor() / 10000.0;
        let expected_result = expected_order_size * input_signal_strength.0 as f64;

        assert_ne!(actual_result, 0.0);
        assert_eq!(actual_result, expected_result)
    }

    #[test]
    fn should_allocate_order_to_enter_short_position_with_correct_quantity() {
        let default_order_value = 1000.0;
        let allocator = DefaultAllocator {
            default_order_value,
        };

        let order_close = 10.0;
        let mut input_order = order_event();
        input_order.market_meta.close = order_close;
        input_order.decision = Decision::Short;

        let input_signal_strength = SignalStrength(1.0);

        allocator.allocate_order(&mut input_order, None, input_signal_strength);

        let actual_result = input_order.quantity;
        let expected_result = -(default_order_value / order_close) * input_signal_strength.0 as f64;

        assert_eq!(actual_result, expected_result)
    }

    #[test]
    fn should_allocate_order_to_enter_short_position_with_with_non_zero_quantity() {
        let default_order_value = 200.0;
        let allocator = DefaultAllocator {
            default_order_value,
        };

        let order_close = 226.753403;
        let mut input_order = order_event();
        input_order.market_meta.close = order_close;
        input_order.decision = Decision::Short;

        let input_signal_strength = SignalStrength(1.0);

        allocator.allocate_order(&mut input_order, None, input_signal_strength);

        let actual_result = input_order.quantity;
        let expected_order_size = ((default_order_value / order_close) * 10000.0).floor() / 10000.0;
        let expected_result = -expected_order_size * input_signal_strength.0;

        assert_ne!(actual_result, 0.0);
        assert_eq!(actual_result, expected_result)
    }
}
