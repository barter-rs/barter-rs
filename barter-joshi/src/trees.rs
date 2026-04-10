//! Binomial tree option pricing.
//!
//! Implements the Cox-Ross-Rubinstein (CRR) binomial tree from Joshi Chapter 7.7.

use crate::payoff::PayOff;

/// Price a European option using the CRR binomial tree.
///
/// # Arguments
/// * `payoff` — option payoff at expiry
/// * `spot` — current spot price
/// * `rate` — risk-free rate (annualised)
/// * `vol` — volatility (annualised)
/// * `expiry` — time to expiry (years)
/// * `steps` — number of time steps
pub fn binomial_european<P: PayOff>(
    payoff: &P,
    spot: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    steps: usize,
) -> f64 {
    let dt = expiry / steps as f64;
    let u = (vol * dt.sqrt()).exp();
    let d = 1.0 / u;
    let discount = (-rate * dt).exp();
    let p = ((rate * dt).exp() - d) / (u - d); // risk-neutral probability
    let q = 1.0 - p;

    // Terminal payoffs at step N
    let mut values: Vec<f64> = (0..=steps)
        .map(|i| {
            let spot_t = spot * u.powi(i as i32) * d.powi((steps - i) as i32);
            payoff.payoff(spot_t)
        })
        .collect();

    // Backward induction
    for step in (0..steps).rev() {
        for i in 0..=step {
            values[i] = discount * (p * values[i + 1] + q * values[i]);
        }
    }

    values[0]
}

/// Price an American option using the CRR binomial tree.
///
/// At each node, the holder can exercise early if the exercise value
/// exceeds the continuation value.
pub fn binomial_american<P: PayOff>(
    payoff: &P,
    spot: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    steps: usize,
) -> f64 {
    let dt = expiry / steps as f64;
    let u = (vol * dt.sqrt()).exp();
    let d = 1.0 / u;
    let discount = (-rate * dt).exp();
    let p = ((rate * dt).exp() - d) / (u - d);
    let q = 1.0 - p;

    // Terminal payoffs
    let mut values: Vec<f64> = (0..=steps)
        .map(|i| {
            let spot_t = spot * u.powi(i as i32) * d.powi((steps - i) as i32);
            payoff.payoff(spot_t)
        })
        .collect();

    // Backward induction with early exercise check
    for step in (0..steps).rev() {
        for i in 0..=step {
            let continuation = discount * (p * values[i + 1] + q * values[i]);
            let spot_at_node = spot * u.powi(i as i32) * d.powi((step - i) as i32);
            let exercise = payoff.payoff(spot_at_node);
            values[i] = continuation.max(exercise);
        }
    }

    values[0]
}

/// Compute the tree-based delta (finite difference of first two nodes).
pub fn binomial_delta<P: PayOff>(
    payoff: &P,
    spot: f64,
    rate: f64,
    vol: f64,
    expiry: f64,
    steps: usize,
) -> f64 {
    let dt = expiry / steps as f64;
    let u = (vol * dt.sqrt()).exp();
    let d = 1.0 / u;
    let discount = (-rate * dt).exp();
    let p = ((rate * dt).exp() - d) / (u - d);
    let q = 1.0 - p;

    // Terminal payoffs
    let mut values: Vec<f64> = (0..=steps)
        .map(|i| {
            let spot_t = spot * u.powi(i as i32) * d.powi((steps - i) as i32);
            payoff.payoff(spot_t)
        })
        .collect();

    // Backward induction to step 1
    for step in (1..steps).rev() {
        for i in 0..=step {
            values[i] = discount * (p * values[i + 1] + q * values[i]);
        }
    }

    // Delta = (V_up - V_down) / (S_up - S_down)
    let v_up = discount * (p * values[2] + q * values[1]);
    let v_down = discount * (p * values[1] + q * values[0]);
    (v_up - v_down) / (spot * u - spot * d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payoff::{Call, Put};

    #[test]
    fn test_european_call_converges_to_bs() {
        let call = Call::new(100.0);
        let tree_price = binomial_european(&call, 100.0, 0.05, 0.2, 1.0, 500);
        // BS price ≈ 10.4506
        assert!(
            (tree_price - 10.4506).abs() < 0.1,
            "tree call price {tree_price} far from BS 10.4506"
        );
    }

    #[test]
    fn test_european_put_converges_to_bs() {
        let put = Put::new(100.0);
        let tree_price = binomial_european(&put, 100.0, 0.05, 0.2, 1.0, 500);
        // BS put ≈ 5.5735
        assert!(
            (tree_price - 5.5735).abs() < 0.1,
            "tree put price {tree_price} far from BS 5.5735"
        );
    }

    #[test]
    fn test_american_put_greater_than_european() {
        let put = Put::new(100.0);
        let euro = binomial_european(&put, 100.0, 0.05, 0.2, 1.0, 200);
        let amer = binomial_american(&put, 100.0, 0.05, 0.2, 1.0, 200);
        assert!(
            amer >= euro - 0.001,
            "American put ({amer}) should be >= European ({euro})"
        );
    }

    #[test]
    fn test_american_call_equals_european() {
        // For non-dividend-paying stock, American call = European call
        let call = Call::new(100.0);
        let euro = binomial_european(&call, 100.0, 0.05, 0.2, 1.0, 200);
        let amer = binomial_american(&call, 100.0, 0.05, 0.2, 1.0, 200);
        assert!(
            (amer - euro).abs() < 0.01,
            "American call ({amer}) should ≈ European ({euro}) for non-div stock"
        );
    }
}
