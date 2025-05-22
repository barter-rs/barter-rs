use derive_more::Constructor;
use rust_decimal::Decimal;
use jackbot_execution::order::request::OrderRequestOpen;

/// Jackpot order that specifies isolated leverage and a maximum allowable loss.
#[derive(Debug, Clone, Constructor)]
pub struct JackpotOrder<ExchangeKey, InstrumentKey> {
    /// Order request to be sent to the exchange.
    pub request: OrderRequestOpen<ExchangeKey, InstrumentKey>,
    /// Isolated leverage to apply to the position.
    pub leverage: Decimal,
    /// Maximum loss tolerated for this order.
    pub ticket_loss: Decimal,
}

/// Manages jackpot orders and validates requests before submission.
#[derive(Debug, Clone, Default)]
pub struct JackpotOrderManager<ExchangeKey, InstrumentKey> {
    active: Vec<JackpotOrder<ExchangeKey, InstrumentKey>>,
}

impl<ExchangeKey, InstrumentKey> JackpotOrderManager<ExchangeKey, InstrumentKey> {
    /// Add a jackpot order to be tracked. Returns an error if leverage or ticket
    /// loss are invalid.
    pub fn add(&mut self, order: JackpotOrder<ExchangeKey, InstrumentKey>) -> Result<(), String> {
        if order.leverage <= Decimal::ONE {
            return Err("leverage must be greater than 1".into());
        }
        if order.ticket_loss <= Decimal::ZERO {
            return Err("ticket loss must be positive".into());
        }
        self.active.push(order);
        Ok(())
    }

    /// Retrieve the ticket loss configured for an instrument, if any.
    pub fn ticket_loss(&self, instrument: &InstrumentKey) -> Option<Decimal>
    where
        InstrumentKey: PartialEq,
    {
        self.active
            .iter()
            .find(|o| &o.request.key.instrument == instrument)
            .map(|o| o.ticket_loss)
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackbot_execution::order::{OrderKey, OrderKind, TimeInForce, id::{ClientOrderId, StrategyId}, request::{RequestOpen}};
    use jackbot_instrument::Side;
    use rust_decimal_macros::dec;

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
                kind: OrderKind::Market,
                time_in_force: TimeInForce::GoodUntilCancelled { post_only: false },
            },
        }
    }

    #[test]
    fn test_add_invalid_leverage() {
        let mut manager: JackpotOrderManager<u8, u8> = JackpotOrderManager::default();
        let order = JackpotOrder::new(sample_request(dec!(100)), dec!(1), dec!(10));
        assert!(manager.add(order).is_err());
    }

    #[test]
    fn test_ticket_loss_lookup() {
        let mut manager: JackpotOrderManager<u8, u8> = JackpotOrderManager::default();
        let order = JackpotOrder::new(sample_request(dec!(100)), dec!(100), dec!(10));
        assert!(manager.add(order).is_ok());
        assert_eq!(manager.ticket_loss(&0), Some(dec!(10)));
    }
}
