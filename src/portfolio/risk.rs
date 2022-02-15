use crate::portfolio::order::{OrderEvent, OrderType};
use serde::Deserialize;

/// Evaluates the risk associated with an [`OrderEvent`] to determine if it should be actioned. It
/// can also amend the order (eg/ [`OrderType`]) to better fit the risk strategy required for
/// profitability.
pub trait OrderEvaluator {
    const DEFAULT_ORDER_TYPE: OrderType;

    /// May return an amended [`OrderEvent`] if the associated risk is appropriate. Returns `None`
    /// if the risk is too high.
    fn evaluate_order(&self, order: OrderEvent) -> Option<OrderEvent>;
}

/// Default risk manager that implements [`OrderEvaluator`].
#[derive(Copy, Debug, Deserialize)]
pub struct DefaultRisk {}

impl OrderEvaluator for DefaultRisk {
    const DEFAULT_ORDER_TYPE: OrderType = OrderType::Market;

    fn evaluate_order(&self, mut order: OrderEvent) -> Option<OrderEvent> {
        if self.risk_too_high(&order) {
            return None
        }
        order.order_type = DefaultRisk::DEFAULT_ORDER_TYPE;
        Some(order)
    }
}

impl DefaultRisk {
    fn risk_too_high(&self, _: &OrderEvent) -> bool {
        false
    }
}