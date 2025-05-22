use jackbot_execution::market_making::{
    FlowToxicityDetector, Quote, QuoteRefresher, TradeSide, reactive_adjust, predictive_adjust,
};
use Jackbot::market_maker::InventorySkewQuoter;
use chrono::{Duration, TimeZone, Utc};
use rust_decimal_macros::dec;

#[test]
fn test_toxic_flow_and_refresh() {
    let detector = FlowToxicityDetector::new(dec!(0.6));
    let trades = vec![(TradeSide::Buy, dec!(7)), (TradeSide::Buy, dec!(3))];
    assert!(detector.is_toxic(&trades));

    let mut refresher = QuoteRefresher::new(Duration::seconds(10));
    let t0 = Utc.timestamp_opt(0, 0).unwrap();
    assert!(refresher.needs_refresh(t0));
    refresher.record_refresh(t0);
    assert!(!refresher.needs_refresh(t0 + Duration::seconds(5)));
    assert!(refresher.needs_refresh(t0 + Duration::seconds(11)));
}

#[test]
fn test_reactive_predictive_with_inventory() {
    let quoter = InventorySkewQuoter::new(dec!(2), dec!(0.5));
    let base = quoter.quote(dec!(100), dec!(0.2));
    let quote = Quote::new(base.bid_price, base.ask_price);

    let reactive = reactive_adjust(quote, TradeSide::Buy, dec!(1));
    assert_eq!(reactive.bid_price, quote.bid_price + dec!(1));

    let predicted = predictive_adjust(reactive, dec!(105));
    let spread = reactive.ask_price - reactive.bid_price;
    assert_eq!(predicted.ask_price - predicted.bid_price, spread);
}
