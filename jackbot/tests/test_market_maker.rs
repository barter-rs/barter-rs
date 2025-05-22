use Jackbot::market_maker::{InventorySkewQuoter, optimize_spread, PerformanceTracker, RiskControls};
use rust_decimal_macros::dec;

#[test]
fn test_quotes_and_risk() {
    let quoter = InventorySkewQuoter::new(dec!(2), dec!(0.5));
    let quote = quoter.quote(dec!(100), dec!(0.2));
    assert_eq!(quote.bid_price, dec!(100) - dec!(1) - dec!(0.1));
    assert_eq!(quote.ask_price, dec!(100) + dec!(1) - dec!(0.1));

    let risk = RiskControls::new(dec!(0.25));
    assert!(risk.check_inventory(dec!(0.2)));
    assert!(!risk.check_inventory(dec!(0.3)));
}

#[test]
fn test_perf_and_spread() {
    let s = optimize_spread(dec!(1), dec!(0.5), dec!(1.5));
    assert_eq!(s, dec!(1.5));

    let mut tracker = PerformanceTracker::default();
    tracker.record_trade(dec!(1));
    tracker.record_trade(dec!(-0.5));
    assert_eq!(tracker.trades(), 2);
    assert_eq!(tracker.realised_pnl(), dec!(0.5));
}

