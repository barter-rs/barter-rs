use Jackbot::engine::state::position::Position;
use jackbot_execution::trade::{Trade, TradeId, AssetFees};
use jackbot_execution::order::id::{OrderId, StrategyId};
use jackbot_instrument::instrument::name::InstrumentNameInternal;
use jackbot_instrument::Side;
use chrono::{DateTime, Utc};
use rust_decimal_macros::dec;

#[test]
fn test_position_loss_exceeded() {
    let trade = Trade {
        id: TradeId::new("t1"),
        order_id: OrderId::new("o1"),
        instrument: InstrumentNameInternal::new("BTC-USD"),
        strategy: StrategyId::new("s1"),
        time_exchange: DateTime::<Utc>::MIN_UTC,
        side: Side::Buy,
        price: dec!(100.0),
        quantity: dec!(1.0),
        fees: AssetFees::quote_fees(dec!(0.0)),
    };
    let mut position = Position::from(&trade);
    position.update_pnl_unrealised(dec!(50.0)); // unrealised = -50
    assert!(position.loss_exceeded(dec!(40.0)));
    assert!(!position.loss_exceeded(dec!(60.0)));
}
