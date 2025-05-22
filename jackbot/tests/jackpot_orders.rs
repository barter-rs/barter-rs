use Jackbot::engine::state::order::jackpot::{JackpotOrder, JackpotOrderManager};
use Jackbot::engine::state::position::Position;
use jackbot_execution::order::{OrderKey, OrderKind, TimeInForce, id::{ClientOrderId, StrategyId}, request::{OrderRequestOpen, RequestOpen}};
use jackbot_execution::trade::{Trade, TradeId, AssetFees};
use jackbot_instrument::{instrument::name::InstrumentNameInternal, Side};
use chrono::{DateTime, Utc};
use rust_decimal_macros::dec;

fn sample_request(price: rust_decimal::Decimal) -> OrderRequestOpen<u8,u8> {
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
fn test_loss_controlled() {
    let mut manager: JackpotOrderManager<u8,u8> = JackpotOrderManager::default();
    let order = JackpotOrder::new(sample_request(dec!(100)), dec!(50), dec!(20));
    manager.add(order).unwrap();

    let trade = Trade {
        id: TradeId::new("t1"),
        order_id: ClientOrderId::default().into(),
        instrument: InstrumentNameInternal::new("BTC-USD"),
        strategy: StrategyId::new("s1"),
        time_exchange: DateTime::<Utc>::MIN_UTC,
        side: Side::Buy,
        price: dec!(100),
        quantity: dec!(1),
        fees: AssetFees::quote_fees(dec!(0)),
    };
    let mut position = Position::from(&trade);
    position.update_pnl_unrealised(dec!(70)); // unrealised = -30
    let ticket = manager.ticket_loss(&0).unwrap();
    assert!(position.loss_exceeded(ticket));
}
