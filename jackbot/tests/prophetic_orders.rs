use jackbot::engine::state::order::prophetic::{PropheticOrder, PropheticOrderManager};
use jackbot_execution::order::{OrderKey, OrderKind, TimeInForce, id::{ClientOrderId, StrategyId}, request::{OrderRequestOpen, RequestOpen}};
use jackbot_instrument::Side;
use chrono::{DateTime, Utc};
use rust_decimal_macros::dec;
use rust_decimal::Decimal;

fn sample_request(price: Decimal) -> OrderRequestOpen<u8,u8> {
    OrderRequestOpen {
        key: OrderKey {
            exchange: 0,
            instrument: 0,
            strategy: StrategyId::unknown(),
            cid: ClientOrderId::default(),
        },
        state: RequestOpen {
            side: Side::Buy,
            price,
            quantity: dec!(1),
            kind: OrderKind::Limit,
            time_in_force: TimeInForce::GoodUntilCancelled { post_only: true },
        },
    }
}

#[test]
fn test_auto_place_when_in_range() {
    let mut manager: PropheticOrderManager<u8,u8> = PropheticOrderManager::default();
    let order = PropheticOrder::new(sample_request(dec!(100)), DateTime::<Utc>::MIN_UTC);
    manager.add(order);

    // price far from order
    assert!(manager.drain_in_range(dec!(50), dec!(5)).is_empty());
    // price enters range
    let orders = manager.drain_in_range(dec!(98), dec!(5));
    assert_eq!(orders.len(), 1);
    assert!(manager.is_empty());
}
