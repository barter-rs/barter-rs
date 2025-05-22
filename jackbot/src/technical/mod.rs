pub fn sma(period: usize, data: &[f64]) -> Vec<f64> {
    if period == 0 || data.len() < period {
        return Vec::new();
    }
    data.windows(period)
        .map(|w| w.iter().sum::<f64>() / period as f64)
        .collect()
}

pub fn ema(period: usize, data: &[f64]) -> Vec<f64> {
    if period == 0 || data.is_empty() {
        return Vec::new();
    }
    let k = 2.0 / (period as f64 + 1.0);
    let mut out = Vec::with_capacity(data.len());
    let mut ema_prev = data[0];
    out.push(ema_prev);
    for &val in &data[1..] {
        ema_prev = val * k + ema_prev * (1.0 - k);
        out.push(ema_prev);
    }
    out
}

pub fn rsi(period: usize, data: &[f64]) -> Vec<f64> {
    if period == 0 || data.len() <= period {
        return Vec::new();
    }
    let mut gains = 0.0;
    let mut losses = 0.0;
    for i in 1..=period {
        let diff = data[i] - data[i - 1];
        if diff >= 0.0 {
            gains += diff;
        } else {
            losses -= diff;
        }
    }
    let mut avg_gain = gains / period as f64;
    let mut avg_loss = losses / period as f64;
    let mut out = Vec::with_capacity(data.len() - period);
    let mut rs = if avg_loss == 0.0 { f64::INFINITY } else { avg_gain / avg_loss };
    out.push(100.0 - 100.0 / (1.0 + rs));
    for i in period + 1..data.len() {
        let diff = data[i] - data[i - 1];
        if diff >= 0.0 {
            avg_gain = (avg_gain * (period as f64 - 1.0) + diff) / period as f64;
            avg_loss = (avg_loss * (period as f64 - 1.0)) / period as f64;
        } else {
            avg_gain = (avg_gain * (period as f64 - 1.0)) / period as f64;
            avg_loss = (avg_loss * (period as f64 - 1.0) - diff) / period as f64;
        }
        rs = if avg_loss == 0.0 { f64::INFINITY } else { avg_gain / avg_loss };
        out.push(100.0 - 100.0 / (1.0 + rs));
    }
    out
}
