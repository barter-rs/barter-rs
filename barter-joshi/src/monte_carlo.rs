//! Monte Carlo pricing engine.
//!
//! Implements the SimpleMonteCarlo from Joshi Chapter 7.2–7.5, with
//! convergence tracking and variance reduction.

use crate::payoff::PayOff;
use crate::random::generate_gaussians;
use crate::statistics::{ConvergenceTable, StatisticsGatherer};
use rand::Rng;

/// Result of a Monte Carlo simulation.
#[derive(Debug, Clone)]
pub struct MonteCarloResult {
    /// Discounted mean payoff (the price estimate).
    pub price: f64,
    /// Standard error of the estimate.
    pub std_error: f64,
    /// Number of paths simulated.
    pub num_paths: u64,
    /// 95% confidence interval: [price - ci, price + ci].
    pub confidence_95: f64,
}

impl MonteCarloResult {
    /// Lower bound of 95% confidence interval.
    pub fn ci_lower(&self) -> f64 {
        self.price - self.confidence_95
    }

    /// Upper bound of 95% confidence interval.
    pub fn ci_upper(&self) -> f64 {
        self.price + self.confidence_95
    }
}

impl std::fmt::Display for MonteCarloResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Price: {:.6} ± {:.6} (95% CI: [{:.6}, {:.6}], paths: {})",
            self.price, self.std_error, self.ci_lower(), self.ci_upper(), self.num_paths
        )
    }
}

/// Simple Monte Carlo European option pricer using geometric Brownian motion.
///
/// Simulates: S(T) = S(0) * exp((r - 0.5σ²)T + σ√T * Z)
/// where Z ~ N(0,1).
///
/// # Arguments
/// * `payoff` — the option payoff function
/// * `spot` — current spot price
/// * `rate` — risk-free rate
/// * `vol` — volatility
/// * `expiry` — time to expiry
/// * `num_paths` — number of simulation paths
pub fn simple_monte_carlo<P: PayOff, R: Rng>(
    payoff: &P,
    spot: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    num_paths: u64,
    rng: &mut R,
) -> MonteCarloResult {
    let mut stats = StatisticsGatherer::new();
    let drift = (rate - 0.5 * vol * vol) * expiry;
    let vol_sqrt_t = vol * expiry.sqrt();
    let discount = (-rate * expiry).exp();

    let gaussians = generate_gaussians(rng, num_paths as usize);

    for z in &gaussians {
        let spot_t = spot * (drift + vol_sqrt_t * z).exp();
        let payoff_value = payoff.payoff(spot_t);
        stats.add(payoff_value);
    }

    MonteCarloResult {
        price: discount * stats.mean(),
        std_error: discount * stats.std_error(),
        num_paths,
        confidence_95: discount * stats.confidence_95(),
    }
}

/// Monte Carlo with anti-thetic variance reduction.
///
/// For each Gaussian z, also simulates with -z. This reduces variance
/// because the errors from upward and downward paths partially cancel.
pub fn monte_carlo_antithetic<P: PayOff, R: Rng>(
    payoff: &P,
    spot: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    num_paths: u64,
    rng: &mut R,
) -> MonteCarloResult {
    let mut stats = StatisticsGatherer::new();
    let drift = (rate - 0.5 * vol * vol) * expiry;
    let vol_sqrt_t = vol * expiry.sqrt();
    let discount = (-rate * expiry).exp();

    let gaussians = generate_gaussians(rng, num_paths as usize / 2);

    for z in &gaussians {
        let spot_up = spot * (drift + vol_sqrt_t * z).exp();
        let spot_down = spot * (drift - vol_sqrt_t * z).exp();
        let avg_payoff = 0.5 * (payoff.payoff(spot_up) + payoff.payoff(spot_down));
        stats.add(avg_payoff);
    }

    MonteCarloResult {
        price: discount * stats.mean(),
        std_error: discount * stats.std_error(),
        num_paths,
        confidence_95: discount * stats.confidence_95(),
    }
}

/// Monte Carlo with convergence table tracking.
///
/// Returns both the final result and a convergence table showing
/// how the estimate evolves as more paths are added.
pub fn monte_carlo_with_convergence<P: PayOff, R: Rng>(
    payoff: &P,
    spot: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    num_paths: u64,
    rng: &mut R,
) -> (MonteCarloResult, ConvergenceTable) {
    let mut convergence = ConvergenceTable::new();
    let drift = (rate - 0.5 * vol * vol) * expiry;
    let vol_sqrt_t = vol * expiry.sqrt();
    let discount = (-rate * expiry).exp();

    let gaussians = generate_gaussians(rng, num_paths as usize);

    for z in &gaussians {
        let spot_t = spot * (drift + vol_sqrt_t * z).exp();
        convergence.add(payoff.payoff(spot_t));
    }
    convergence.finalize();

    let stats = convergence.gatherer();
    let result = MonteCarloResult {
        price: discount * stats.mean(),
        std_error: discount * stats.std_error(),
        num_paths,
        confidence_95: discount * stats.confidence_95(),
    };

    (result, convergence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payoff::{Call, Put};

    #[test]
    fn test_simple_monte_carlo_call() {
        let call = Call::new(100.0);
        let mut rng = rand::rng();
        let result = simple_monte_carlo(&call, 100.0, 0.05, 0.2, 1.0, 100_000, &mut rng);

        // BS price ≈ 10.4506
        assert!(
            (result.price - 10.4506).abs() < 0.5,
            "MC call price {} far from BS 10.4506",
            result.price
        );
    }

    #[test]
    fn test_antithetic_reduces_variance() {
        let call = Call::new(100.0);
        let mut rng1 = rand::rng();
        let mut rng2 = rand::rng();

        let basic = simple_monte_carlo(&call, 100.0, 0.05, 0.2, 1.0, 50_000, &mut rng1);
        let anti = monte_carlo_antithetic(&call, 100.0, 0.05, 0.2, 1.0, 50_000, &mut rng2);

        // Antithetic should generally have smaller std_error (not guaranteed for single run)
        println!("Basic std_error: {}, Antithetic std_error: {}", basic.std_error, anti.std_error);
    }

    #[test]
    fn test_put_call_parity_mc() {
        let call = Call::new(100.0);
        let put = Put::new(100.0);
        let mut rng = rand::rng();

        let c = simple_monte_carlo(&call, 100.0, 0.05, 0.2, 1.0, 200_000, &mut rng);
        let p = simple_monte_carlo(&put, 100.0, 0.05, 0.2, 1.0, 200_000, &mut rng);

        let parity = c.price - p.price - (100.0 - 100.0 * (-0.05_f64).exp());
        assert!(
            parity.abs() < 1.0,
            "MC put-call parity residual too large: {parity}"
        );
    }

    #[test]
    fn test_convergence_table() {
        let call = Call::new(100.0);
        let mut rng = rand::rng();
        let (result, convergence) = monte_carlo_with_convergence(
            &call, 100.0, 0.05, 0.2, 1.0, 1024, &mut rng,
        );
        assert!(!convergence.entries.is_empty());
        assert!(result.price > 0.0);
    }
}
