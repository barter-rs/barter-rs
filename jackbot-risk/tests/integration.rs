use jackbot_risk::{
    alert::{VecAlertHook, RiskViolation},
    exposure::ExposureTracker,
    drawdown::DrawdownTracker,
    correlation::CorrelationMatrix,
    volatility::VolatilityScaler,
};
use jackbot_instrument::instrument::InstrumentIndex;
use rust_decimal_macros::dec;

#[test]
fn exposure_alert_triggered() {
    let mut tracker: ExposureTracker<InstrumentIndex> = ExposureTracker::new();
    tracker.update(InstrumentIndex(0), dec!(50));
    let alerts = VecAlertHook::default();
    tracker.check_limit(InstrumentIndex(0), dec!(20), &alerts);
    assert!(matches!(alerts.alerts.lock().pop().unwrap(), RiskViolation::ExposureLimit { .. }));
}

#[test]
fn drawdown_alert_triggered() {
    let mut tracker: DrawdownTracker<InstrumentIndex> = DrawdownTracker::new();
    tracker.update_pnl(InstrumentIndex(0), dec!(100));
    tracker.update_pnl(InstrumentIndex(0), dec!(-60));
    let alerts = VecAlertHook::default();
    tracker.check_limit(InstrumentIndex(0), dec!(0.3), &alerts);
    assert!(matches!(alerts.alerts.lock().pop().unwrap(), RiskViolation::DrawdownLimit { .. }));
}

#[test]
fn correlation_alert_triggered() {
    let mut corr: CorrelationMatrix<InstrumentIndex> = CorrelationMatrix::new();
    corr.set_limit(InstrumentIndex(0), InstrumentIndex(1), dec!(40));
    let alerts = VecAlertHook::default();
    corr.check_limit(InstrumentIndex(0), InstrumentIndex(1), dec!(50), &alerts);
    assert!(matches!(alerts.alerts.lock().pop().unwrap(), RiskViolation::CorrelationLimit { .. }));
}

#[test]
fn volatility_scaler_adjusts_position() {
    let scaler = VolatilityScaler::new(dec!(0.02), dec!(0.5), dec!(2));
    let adjusted = scaler.adjust_position(dec!(10), dec!(0.04));
    assert_eq!(adjusted, dec!(5));
}
