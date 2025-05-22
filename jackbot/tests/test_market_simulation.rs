use Jackbot::backtest::simulation::{MarketSimulator, SimulationConfig};
use rust_decimal::Decimal;

#[test]
fn test_market_simulator_execute() {
    let sim = MarketSimulator::new(SimulationConfig { latency: std::time::Duration::from_millis(50), slippage_bps: 10.0, fee_bps: 5.0 });
    let res = sim.execute(Decimal::new(100, 0), Decimal::new(2, 0));
    // executed price should include slippage of 0.1%
    assert_eq!(res.executed_price, Decimal::new(100, 0) + Decimal::new(1, 1));
    // fee should be based on executed price * quantity * fee_bps
    let expected_fee = res.executed_price * Decimal::new(2,0) * Decimal::new(5,2) / Decimal::new(100,0);
    assert_eq!(res.fee, expected_fee);
    assert_eq!(sim.latency(), std::time::Duration::from_millis(50));
}
