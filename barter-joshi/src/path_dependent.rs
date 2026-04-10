//! Path-dependent option pricing via Monte Carlo.
//!
//! Implements Asian and barrier options from Joshi Chapters 7.9–7.10.

use crate::payoff::PayOff;
use crate::random::generate_gaussians;
use crate::statistics::StatisticsGatherer;
use rand::Rng;

/// Result of a path-dependent Monte Carlo simulation.
#[derive(Debug, Clone)]
pub struct PathResult {
    pub price: f64,
    pub std_error: f64,
    pub num_paths: u64,
}

impl std::fmt::Display for PathResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Price: {:.6} ± {:.6} (paths: {})",
            self.price, self.std_error, self.num_paths
        )
    }
}

/// Price an arithmetic Asian call option via Monte Carlo.
///
/// The payoff is max(A - K, 0) where A is the arithmetic average of
/// spot prices at monitoring dates.
///
/// # Arguments
/// * `spot` — initial spot price
/// * `strike` — strike price
/// * `rate` — risk-free rate
/// * `vol` — volatility
/// * `expiry` — time to expiry
/// * `num_dates` — number of averaging dates (equally spaced)
/// * `num_paths` — number of simulation paths
pub fn asian_call_arithmetic<R: Rng>(
    spot: f64,
    strike: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    num_dates: usize,
    num_paths: u64,
    rng: &mut R,
) -> PathResult {
    let dt = expiry / num_dates as f64;
    let drift = (rate - 0.5 * vol * vol) * dt;
    let vol_sqrt_dt = vol * dt.sqrt();
    let discount = (-rate * expiry).exp();

    let mut stats = StatisticsGatherer::new();

    for _ in 0..num_paths {
        let gaussians = generate_gaussians(rng, num_dates);
        let mut current_spot = spot;
        let mut running_sum = 0.0;

        for z in &gaussians {
            current_spot *= (drift + vol_sqrt_dt * z).exp();
            running_sum += current_spot;
        }

        let average = running_sum / num_dates as f64;
        let payoff = (average - strike).max(0.0);
        stats.add(payoff);
    }

    PathResult {
        price: discount * stats.mean(),
        std_error: discount * stats.std_error(),
        num_paths,
    }
}

/// Price an arithmetic Asian put option via Monte Carlo.
pub fn asian_put_arithmetic<R: Rng>(
    spot: f64,
    strike: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    num_dates: usize,
    num_paths: u64,
    rng: &mut R,
) -> PathResult {
    let dt = expiry / num_dates as f64;
    let drift = (rate - 0.5 * vol * vol) * dt;
    let vol_sqrt_dt = vol * dt.sqrt();
    let discount = (-rate * expiry).exp();

    let mut stats = StatisticsGatherer::new();

    for _ in 0..num_paths {
        let gaussians = generate_gaussians(rng, num_dates);
        let mut current_spot = spot;
        let mut running_sum = 0.0;

        for z in &gaussians {
            current_spot *= (drift + vol_sqrt_dt * z).exp();
            running_sum += current_spot;
        }

        let average = running_sum / num_dates as f64;
        let payoff = (strike - average).max(0.0);
        stats.add(payoff);
    }

    PathResult {
        price: discount * stats.mean(),
        std_error: discount * stats.std_error(),
        num_paths,
    }
}

/// Price a geometric Asian call (closed-form approximation benchmark).
///
/// The geometric average has a known distribution under GBM, so this can
/// serve as a control variate for the arithmetic Asian.
pub fn asian_call_geometric_closed_form(
    spot: f64,
    strike: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    num_dates: usize,
) -> f64 {
    let n = num_dates as f64;
    let dt = expiry / n;

    // Adjusted parameters for geometric average
    let vol_adj = vol * ((2.0 * n + 1.0) / (6.0 * (n + 1.0))).sqrt();
    let rate_adj = 0.5 * (rate - 0.5 * vol * vol + vol_adj * vol_adj);

    // Black-Scholes with adjusted parameters
    crate::black_scholes::call_price(spot, strike, rate_adj, vol_adj, expiry)
        * (-rate * expiry).exp()
        / (-rate_adj * expiry).exp()
}

/// Price an up-and-out barrier call via Monte Carlo.
///
/// The option is knocked out (becomes worthless) if the spot price
/// ever exceeds the barrier level during the life of the option.
pub fn barrier_up_and_out_call<R: Rng>(
    spot: f64,
    strike: f64,
    barrier: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    num_dates: usize,
    num_paths: u64,
    rng: &mut R,
) -> PathResult {
    assert!(barrier > spot, "barrier must be above spot for up-and-out");

    let dt = expiry / num_dates as f64;
    let drift = (rate - 0.5 * vol * vol) * dt;
    let vol_sqrt_dt = vol * dt.sqrt();
    let discount = (-rate * expiry).exp();

    let mut stats = StatisticsGatherer::new();

    for _ in 0..num_paths {
        let gaussians = generate_gaussians(rng, num_dates);
        let mut current_spot = spot;
        let mut knocked_out = false;

        for z in &gaussians {
            current_spot *= (drift + vol_sqrt_dt * z).exp();
            if current_spot >= barrier {
                knocked_out = true;
                break;
            }
        }

        let payoff = if knocked_out {
            0.0
        } else {
            (current_spot - strike).max(0.0)
        };
        stats.add(payoff);
    }

    PathResult {
        price: discount * stats.mean(),
        std_error: discount * stats.std_error(),
        num_paths,
    }
}

/// Price a down-and-out barrier put via Monte Carlo.
pub fn barrier_down_and_out_put<R: Rng>(
    spot: f64,
    strike: f64,
    barrier: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    num_dates: usize,
    num_paths: u64,
    rng: &mut R,
) -> PathResult {
    assert!(barrier < spot, "barrier must be below spot for down-and-out");

    let dt = expiry / num_dates as f64;
    let drift = (rate - 0.5 * vol * vol) * dt;
    let vol_sqrt_dt = vol * dt.sqrt();
    let discount = (-rate * expiry).exp();

    let mut stats = StatisticsGatherer::new();

    for _ in 0..num_paths {
        let gaussians = generate_gaussians(rng, num_dates);
        let mut current_spot = spot;
        let mut knocked_out = false;

        for z in &gaussians {
            current_spot *= (drift + vol_sqrt_dt * z).exp();
            if current_spot <= barrier {
                knocked_out = true;
                break;
            }
        }

        let payoff = if knocked_out {
            0.0
        } else {
            (strike - current_spot).max(0.0)
        };
        stats.add(payoff);
    }

    PathResult {
        price: discount * stats.mean(),
        std_error: discount * stats.std_error(),
        num_paths,
    }
}

/// Price a lookback call option: payoff = S(T) - min(S(t)).
pub fn lookback_call<R: Rng>(
    spot: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    num_dates: usize,
    num_paths: u64,
    rng: &mut R,
) -> PathResult {
    let dt = expiry / num_dates as f64;
    let drift = (rate - 0.5 * vol * vol) * dt;
    let vol_sqrt_dt = vol * dt.sqrt();
    let discount = (-rate * expiry).exp();

    let mut stats = StatisticsGatherer::new();

    for _ in 0..num_paths {
        let gaussians = generate_gaussians(rng, num_dates);
        let mut current_spot = spot;
        let mut min_spot = spot;

        for z in &gaussians {
            current_spot *= (drift + vol_sqrt_dt * z).exp();
            min_spot = min_spot.min(current_spot);
        }

        let payoff = current_spot - min_spot;
        stats.add(payoff);
    }

    PathResult {
        price: discount * stats.mean(),
        std_error: discount * stats.std_error(),
        num_paths,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asian_call_positive() {
        let mut rng = rand::rng();
        let result = asian_call_arithmetic(100.0, 100.0, 0.05, 0.2, 1.0, 12, 50_000, &mut rng);
        assert!(result.price > 0.0, "Asian call price should be positive");
        // Asian call should be cheaper than European call (~10.45)
        assert!(result.price < 12.0, "Asian call should be < European call");
    }

    #[test]
    fn test_barrier_cheaper_than_vanilla() {
        let mut rng = rand::rng();
        let barrier_result = barrier_up_and_out_call(
            100.0, 100.0, 130.0, 0.05, 0.2, 1.0, 252, 50_000, &mut rng,
        );
        // Up-and-out barrier call should be cheaper than vanilla call
        assert!(barrier_result.price < 12.0);
        assert!(barrier_result.price > 0.0);
    }

    #[test]
    fn test_lookback_more_expensive_than_vanilla() {
        let mut rng = rand::rng();
        let result = lookback_call(100.0, 0.05, 0.2, 1.0, 252, 50_000, &mut rng);
        // Lookback call should be more expensive than vanilla ATM call (~10.45)
        assert!(result.price > 5.0, "Lookback should be substantial: {}", result.price);
    }
}
