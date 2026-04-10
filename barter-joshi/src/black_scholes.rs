//! Black-Scholes closed-form pricing and Greeks.
//!
//! Implements the analytical formulae from Joshi Chapters 3–4 and the C++ code
//! in Chapter 7.

use crate::random::{normal_cdf, normal_pdf};

/// Black-Scholes European call price.
///
/// # Arguments
/// * `spot` — current spot price S
/// * `strike` — strike price K
/// * `rate` — risk-free interest rate r (annualised)
/// * `vol` — volatility σ (annualised)
/// * `expiry` — time to expiry T (in years)
pub fn call_price(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (d1, d2) = d1_d2(spot, strike, rate, vol, expiry);
    spot * normal_cdf(d1) - strike * (-rate * expiry).exp() * normal_cdf(d2)
}

/// Black-Scholes European put price.
pub fn put_price(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (d1, d2) = d1_d2(spot, strike, rate, vol, expiry);
    strike * (-rate * expiry).exp() * normal_cdf(-d2) - spot * normal_cdf(-d1)
}

/// Black-Scholes d1 and d2 parameters.
pub fn d1_d2(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> (f64, f64) {
    let vol_sqrt_t = vol * expiry.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * expiry) / vol_sqrt_t;
    let d2 = d1 - vol_sqrt_t;
    (d1, d2)
}

/// Digital (binary) call price: e^{-rT} * N(d2).
pub fn digital_call_price(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (_, d2) = d1_d2(spot, strike, rate, vol, expiry);
    (-rate * expiry).exp() * normal_cdf(d2)
}

/// Digital (binary) put price: e^{-rT} * N(-d2).
pub fn digital_put_price(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (_, d2) = d1_d2(spot, strike, rate, vol, expiry);
    (-rate * expiry).exp() * normal_cdf(-d2)
}

// ---------------------------------------------------------------------------
// Greeks
// ---------------------------------------------------------------------------

/// Delta: ∂C/∂S for a call.
pub fn call_delta(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (d1, _) = d1_d2(spot, strike, rate, vol, expiry);
    normal_cdf(d1)
}

/// Delta: ∂P/∂S for a put.
pub fn put_delta(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    call_delta(spot, strike, rate, vol, expiry) - 1.0
}

/// Gamma: ∂²V/∂S² (same for call and put).
pub fn gamma(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (d1, _) = d1_d2(spot, strike, rate, vol, expiry);
    normal_pdf(d1) / (spot * vol * expiry.sqrt())
}

/// Vega: ∂V/∂σ (same for call and put).
pub fn vega(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (d1, _) = d1_d2(spot, strike, rate, vol, expiry);
    spot * normal_pdf(d1) * expiry.sqrt()
}

/// Theta: ∂C/∂t for a call (negative of time decay).
pub fn call_theta(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (d1, d2) = d1_d2(spot, strike, rate, vol, expiry);
    let term1 = -spot * normal_pdf(d1) * vol / (2.0 * expiry.sqrt());
    let term2 = -rate * strike * (-rate * expiry).exp() * normal_cdf(d2);
    term1 + term2
}

/// Theta: ∂P/∂t for a put.
pub fn put_theta(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (d1, d2) = d1_d2(spot, strike, rate, vol, expiry);
    let term1 = -spot * normal_pdf(d1) * vol / (2.0 * expiry.sqrt());
    let term2 = rate * strike * (-rate * expiry).exp() * normal_cdf(-d2);
    term1 + term2
}

/// Rho: ∂C/∂r for a call.
pub fn call_rho(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (_, d2) = d1_d2(spot, strike, rate, vol, expiry);
    strike * expiry * (-rate * expiry).exp() * normal_cdf(d2)
}

/// Rho: ∂P/∂r for a put.
pub fn put_rho(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let (_, d2) = d1_d2(spot, strike, rate, vol, expiry);
    -strike * expiry * (-rate * expiry).exp() * normal_cdf(-d2)
}

// ---------------------------------------------------------------------------
// Put-Call Parity
// ---------------------------------------------------------------------------

/// Verify put-call parity: C - P = S - K*e^{-rT}.
pub fn put_call_parity_residual(spot: f64, strike: f64, rate: f64, vol: f64, expiry: f64) -> f64 {
    let c = call_price(spot, strike, rate, vol, expiry);
    let p = put_price(spot, strike, rate, vol, expiry);
    (c - p) - (spot - strike * (-rate * expiry).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    #[test]
    fn test_call_put_parity() {
        let residual = put_call_parity_residual(100.0, 100.0, 0.05, 0.2, 1.0);
        assert!(residual.abs() < EPS, "put-call parity violated: {residual}");
    }

    #[test]
    fn test_atm_call_price() {
        // ATM call: S=100, K=100, r=5%, vol=20%, T=1
        let price = call_price(100.0, 100.0, 0.05, 0.2, 1.0);
        assert!((price - 10.4506).abs() < 0.01, "call price: {price}");
    }

    #[test]
    fn test_deep_itm_call_delta() {
        let delta = call_delta(200.0, 100.0, 0.05, 0.2, 1.0);
        assert!(delta > 0.99, "deep ITM call delta should be ~1: {delta}");
    }

    #[test]
    fn test_deep_otm_call_delta() {
        let delta = call_delta(50.0, 100.0, 0.05, 0.2, 1.0);
        assert!(delta < 0.01, "deep OTM call delta should be ~0: {delta}");
    }

    #[test]
    fn test_gamma_positive() {
        let g = gamma(100.0, 100.0, 0.05, 0.2, 1.0);
        assert!(g > 0.0, "gamma should be positive: {g}");
    }

    #[test]
    fn test_vega_positive() {
        let v = vega(100.0, 100.0, 0.05, 0.2, 1.0);
        assert!(v > 0.0, "vega should be positive: {v}");
    }

    #[test]
    fn test_digital_prices_sum_to_discount() {
        let dc = digital_call_price(100.0, 100.0, 0.05, 0.2, 1.0);
        let dp = digital_put_price(100.0, 100.0, 0.05, 0.2, 1.0);
        let discount = (-0.05_f64).exp();
        assert!((dc + dp - discount).abs() < EPS, "digital prices should sum to discount factor");
    }
}
