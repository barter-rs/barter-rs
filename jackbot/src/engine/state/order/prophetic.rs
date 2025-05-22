use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use jackbot_execution::order::request::OrderRequestOpen;
use derive_more::Constructor;

/// Prophetic order that is stored until the market is in range
#[derive(Debug, Clone, Constructor)]
pub struct PropheticOrder<ExchangeKey, InstrumentKey> {
    pub request: OrderRequestOpen<ExchangeKey, InstrumentKey>,
    pub submitted_at: DateTime<Utc>,
}

/// Manager that tracks prophetic orders and checks if they are in range
#[derive(Debug, Clone, Default)]
pub struct PropheticOrderManager<ExchangeKey, InstrumentKey> {
    pending: Vec<PropheticOrder<ExchangeKey, InstrumentKey>>,
}

impl<ExchangeKey, InstrumentKey> PropheticOrderManager<ExchangeKey, InstrumentKey> {
    /// Add a prophetic order to be monitored
    pub fn add(&mut self, order: PropheticOrder<ExchangeKey, InstrumentKey>) {
        // avoid duplicates based on client order id
        if self
            .pending
            .iter()
            .any(|o| o.request.key.cid == order.request.key.cid)
        {
            return;
        }

        self.pending.push(order);
    }

    /// Check pending orders against the given market price and return orders that are now in range
    pub fn drain_in_range(&mut self, market_price: Decimal, range_percent: Decimal) -> Vec<OrderRequestOpen<ExchangeKey, InstrumentKey>> {
        let mut ready = Vec::new();
        self.pending.retain(|order| {
            let diff = (order.request.state.price - market_price).abs();
            let threshold = market_price * range_percent.abs() / Decimal::new(100,0);
            if diff <= threshold {
                ready.push(order.request.clone());
                false
            } else {
                true
            }
        });
        ready
    }

    /// Add an order and immediately return it if already within range of the provided market price.
    /// Orders outside the range are stored for later processing.
    pub fn add_or_place(
        &mut self,
        order: PropheticOrder<ExchangeKey, InstrumentKey>,
        market_price: Decimal,
        range_percent: Decimal,
    ) -> Option<OrderRequestOpen<ExchangeKey, InstrumentKey>> {
        let diff = (order.request.state.price - market_price).abs();
        let threshold = market_price * range_percent.abs() / Decimal::new(100, 0);
        if diff <= threshold {
            Some(order.request)
        } else {
            self.add(order);
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use jackbot_execution::order::{OrderKey, OrderKind, TimeInForce, id::{ClientOrderId, StrategyId}, request::RequestOpen};
    use jackbot_instrument::Side;

    type TestReq = OrderRequestOpen<u8, u8>;

    fn sample_request(price: Decimal) -> TestReq {
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
    fn test_prophetic_order_manager() {
        let mut manager: PropheticOrderManager<u8,u8> = PropheticOrderManager::default();
        let order = PropheticOrder::new(sample_request(dec!(100)), DateTime::<Utc>::MIN_UTC);
        manager.add(order);

        assert!(!manager.is_empty());
        // price far away, should not trigger
        let ready = manager.drain_in_range(dec!(50), dec!(5));
        assert!(ready.is_empty());
        assert!(!manager.is_empty());

        // price moves close enough
        let ready = manager.drain_in_range(dec!(96), dec!(5));
        assert_eq!(ready.len(), 1);
        assert!(manager.is_empty());
    }
}

