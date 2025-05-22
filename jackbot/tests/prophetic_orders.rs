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

#[test]
fn test_add_or_place_and_negative_range() {
    let mut manager: PropheticOrderManager<u8, u8> = PropheticOrderManager::default();
    let price = dec!(100);
    let order = PropheticOrder::new(sample_request(price), DateTime::<Utc>::MIN_UTC);

    // negative range should be treated as positive and order placed immediately
    let out = manager.add_or_place(order.clone(), price, dec!(-5));
    assert!(out.is_some());
    assert!(manager.is_empty());

    // add again but outside range
    let out = manager.add_or_place(order.clone(), dec!(80), dec!(5));
    assert!(out.is_none());
    assert!(!manager.is_empty());

    // duplicate should be ignored
    manager.add(order);
    assert_eq!(manager.pending_count(), 1);

    // price moves in range
    let ready = manager.drain_in_range(dec!(95), dec!(5));
    assert_eq!(ready.len(), 1);
    assert!(manager.is_empty());
}
