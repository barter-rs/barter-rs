use jackbot_ta::{
    indicators::{ExponentialMovingAverage, SimpleMovingAverage},
    patterns::{crossover, Cross},
    signals::{CrossOverSignal, Signal},
};
use rust_decimal_macros::dec;

#[test]
fn sma_and_ema() {
    let mut sma = SimpleMovingAverage::new(3);
    assert_eq!(sma.update(dec!(1)), dec!(1));
    assert_eq!(sma.update(dec!(2)), dec!(1.5));
    assert_eq!(sma.update(dec!(3)), dec!(2));
    assert_eq!(sma.update(dec!(4)), dec!(3));

    let mut ema = ExponentialMovingAverage::new(3);
    assert_eq!(ema.update(dec!(1)), dec!(1));
    let v = ema.update(dec!(2));
    assert!(v > dec!(1));
}

#[test]
fn test_crossover_pattern() {
    let res = crossover(dec!(1), dec!(2), dec!(3), dec!(2));
    assert_eq!(res, Some(Cross::Above));
}

#[test]
fn test_signal_generation() {
    let mut gen = CrossOverSignal::new();
    assert_eq!(gen.update(dec!(1), dec!(2)), None);
    assert_eq!(gen.update(dec!(2), dec!(2)), None);
    assert_eq!(gen.update(dec!(3), dec!(2)), Some(Signal::Buy));
    assert_eq!(gen.update(dec!(2), dec!(3)), Some(Signal::Sell));
}
