//! Numerical root-finding solvers.
//!
//! Implements Newton-Raphson and bisection methods from Joshi Chapter 7.8,
//! primarily for computing implied volatility.

use crate::black_scholes;

/// Bisection root finder.
///
/// Finds x in [lo, hi] such that f(x) ≈ 0.
///
/// # Arguments
/// * `f` — function to find root of
/// * `lo` — lower bound (must have f(lo) * f(hi) < 0)
/// * `hi` — upper bound
/// * `tol` — convergence tolerance
/// * `max_iter` — maximum iterations
pub fn bisection<F: Fn(f64) -> f64>(
    f: &F,
    mut lo: f64,
    mut hi: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, &'static str> {
    let f_lo = f(lo);
    let f_hi = f(hi);

    if f_lo * f_hi > 0.0 {
        return Err("bisection: f(lo) and f(hi) must have opposite signs");
    }

    for _ in 0..max_iter {
        let mid = 0.5 * (lo + hi);
        let f_mid = f(mid);

        if f_mid.abs() < tol || (hi - lo) < tol {
            return Ok(mid);
        }

        if f_lo * f_mid < 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }

    Ok(0.5 * (lo + hi))
}

/// Newton-Raphson root finder.
///
/// Finds x such that f(x) ≈ 0 using the iteration x_{n+1} = x_n - f(x_n)/f'(x_n).
///
/// # Arguments
/// * `f` — function to find root of
/// * `f_prime` — derivative of f
/// * `x0` — initial guess
/// * `tol` — convergence tolerance
/// * `max_iter` — maximum iterations
pub fn newton_raphson<F, G>(
    f: &F,
    f_prime: &G,
    mut x: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, &'static str>
where
    F: Fn(f64) -> f64,
    G: Fn(f64) -> f64,
{
    for _ in 0..max_iter {
        let fx = f(x);
        if fx.abs() < tol {
            return Ok(x);
        }
        let fpx = f_prime(x);
        if fpx.abs() < 1e-15 {
            return Err("newton_raphson: derivative is zero");
        }
        x -= fx / fpx;
    }

    Err("newton_raphson: did not converge")
}

/// Compute implied volatility for a European call using Newton-Raphson.
///
/// Uses vega (∂C/∂σ) as the derivative, starting from an initial guess.
///
/// # Arguments
/// * `market_price` — observed market price of the call
/// * `spot` — current spot price
/// * `strike` — strike price
/// * `rate` — risk-free rate
/// * `expiry` — time to expiry
/// * `initial_vol` — starting guess for volatility (default: 0.2)
/// * `tol` — convergence tolerance
/// * `max_iter` — maximum iterations
pub fn implied_vol_call(
    market_price: f64,
    spot: f64,
    strike: f64,
    rate: f64,
    expiry: f64,
    initial_vol: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, &'static str> {
    let f = |vol: f64| black_scholes::call_price(spot, strike, rate, vol, expiry) - market_price;
    let f_prime = |vol: f64| black_scholes::vega(spot, strike, rate, vol, expiry);

    newton_raphson(&f, &f_prime, initial_vol, tol, max_iter)
}

/// Compute implied volatility for a European put using Newton-Raphson.
pub fn implied_vol_put(
    market_price: f64,
    spot: f64,
    strike: f64,
    rate: f64,
    expiry: f64,
    initial_vol: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, &'static str> {
    let f = |vol: f64| black_scholes::put_price(spot, strike, rate, vol, expiry) - market_price;
    let f_prime = |vol: f64| black_scholes::vega(spot, strike, rate, vol, expiry);

    newton_raphson(&f, &f_prime, initial_vol, tol, max_iter)
}

/// Compute implied volatility using bisection (more robust, slower).
pub fn implied_vol_call_bisection(
    market_price: f64,
    spot: f64,
    strike: f64,
    rate: f64,
    expiry: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, &'static str> {
    let f = |vol: f64| black_scholes::call_price(spot, strike, rate, vol, expiry) - market_price;
    bisection(&f, 0.001, 5.0, tol, max_iter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bisection_sqrt2() {
        let f = |x: f64| x * x - 2.0;
        let root = bisection(&f, 1.0, 2.0, 1e-10, 100).unwrap();
        assert!((root - std::f64::consts::SQRT_2).abs() < 1e-9);
    }

    #[test]
    fn test_newton_raphson_sqrt2() {
        let f = |x: f64| x * x - 2.0;
        let fp = |x: f64| 2.0 * x;
        let root = newton_raphson(&f, &fp, 1.5, 1e-10, 100).unwrap();
        assert!((root - std::f64::consts::SQRT_2).abs() < 1e-9);
    }

    #[test]
    fn test_implied_vol_roundtrip() {
        // Price a call at known vol, then recover the vol
        let true_vol = 0.25;
        let price = black_scholes::call_price(100.0, 100.0, 0.05, true_vol, 1.0);
        let implied = implied_vol_call(price, 100.0, 100.0, 0.05, 1.0, 0.2, 1e-8, 100).unwrap();
        assert!(
            (implied - true_vol).abs() < 1e-6,
            "implied vol {implied} != true vol {true_vol}"
        );
    }

    #[test]
    fn test_implied_vol_bisection_roundtrip() {
        let true_vol = 0.3;
        let price = black_scholes::call_price(100.0, 110.0, 0.03, true_vol, 0.5);
        let implied = implied_vol_call_bisection(price, 100.0, 110.0, 0.03, 0.5, 1e-8, 100).unwrap();
        assert!(
            (implied - true_vol).abs() < 1e-4,
            "bisection implied vol {implied} != true vol {true_vol}"
        );
    }
}
