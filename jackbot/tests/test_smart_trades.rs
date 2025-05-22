use Jackbot::smart_trade::{
    TrailingTakeProfit, ProfitTarget, TrailingStop, MultiLevelStop, MultiLevelTakeProfit,
    SmartTradeSignal,
};
use rust_decimal_macros::dec;

#[test]
fn test_trailing_take_profit() {
    let mut tp = TrailingTakeProfit::new(dec!(10));
    assert_eq!(tp.update(dec!(100)), None);
    assert_eq!(tp.update(dec!(110)), None);
    assert_eq!(tp.update(dec!(105)), None);
    assert_eq!(tp.update(dec!(99)), Some(SmartTradeSignal::TakeProfit(dec!(99))));
    assert_eq!(tp.update(dec!(101)), None);
}

#[test]
fn test_profit_target() {
    let mut pt = ProfitTarget::new(dec!(150));
    assert_eq!(pt.update(dec!(140)), None);
    assert_eq!(pt.update(dec!(150)), Some(SmartTradeSignal::TakeProfit(dec!(150))));
    assert_eq!(pt.update(dec!(160)), None);
}

#[test]
fn test_trailing_stop() {
    let mut ts = TrailingStop::new(dec!(5));
    assert_eq!(ts.update(dec!(100)), None);
    assert_eq!(ts.update(dec!(110)), None);
    assert_eq!(ts.update(dec!(104)), None);
    assert_eq!(ts.update(dec!(103)), Some(SmartTradeSignal::StopLoss(dec!(103))));
    assert_eq!(ts.update(dec!(120)), None);
}

#[test]
fn test_multi_level_stop() {
    let mut ms = MultiLevelStop::new(vec![dec!(90), dec!(80)]);
    assert_eq!(ms.update(dec!(100)), None);
    assert_eq!(ms.update(dec!(89)), Some(SmartTradeSignal::StopLevel(0, dec!(89))));
    assert_eq!(ms.update(dec!(79)), Some(SmartTradeSignal::StopLevel(1, dec!(79))));
    assert_eq!(ms.update(dec!(70)), None);
}

#[test]
fn test_multi_level_take_profit() {
    let mut mtp = MultiLevelTakeProfit::new(vec![dec!(110), dec!(120)]);
    assert_eq!(mtp.update(dec!(100)), None);
    assert_eq!(mtp.update(dec!(110)), Some(SmartTradeSignal::TakeProfit(dec!(110))));
    assert_eq!(mtp.update(dec!(115)), None);
    assert_eq!(mtp.update(dec!(120)), Some(SmartTradeSignal::TakeProfit(dec!(120))));
    assert_eq!(mtp.update(dec!(130)), None);
}
