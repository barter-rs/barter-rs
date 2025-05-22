use Jackbot::technical::{sma, ema, rsi};

#[test]
fn test_sma() {
    let data = [1.0, 2.0, 3.0, 4.0, 5.0];
    let sma3 = sma(3, &data);
    assert_eq!(sma3, vec![2.0, 3.0, 4.0]);
}

#[test]
fn test_ema() {
    let data = [1.0, 2.0, 3.0];
    let ema2 = ema(2, &data);
    assert_eq!(ema2.len(), 3);
    assert!((ema2[0] - 1.0).abs() < 1e-8);
}

#[test]
fn test_rsi() {
    let data = [1.0, 2.0, 3.0, 2.5, 2.0, 3.0];
    let r = rsi(2, &data);
    assert_eq!(r.len(), data.len() - 2);
}
