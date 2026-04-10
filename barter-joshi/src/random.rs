//! Random number generation for Monte Carlo simulation.
//!
//! Implements Gaussian random number generation as described in Joshi Chapter 7.4.
//! Includes Box-Muller transform and anti-thetic sampling.

use rand::Rng;
use rand_distr::{Distribution, StandardNormal};

/// Generate a vector of standard normal random variates.
pub fn generate_gaussians<R: Rng>(rng: &mut R, n: usize) -> Vec<f64> {
    (0..n).map(|_| StandardNormal.sample(rng)).collect()
}

/// Generate Gaussian variates using the Box-Muller transform.
///
/// This is the explicit implementation from Joshi Chapter 7.4.
/// Takes pairs of uniform(0,1) variates and transforms them to N(0,1).
pub fn box_muller<R: Rng>(rng: &mut R, n: usize) -> Vec<f64> {
    let mut result = Vec::with_capacity(n);
    let pairs = (n + 1) / 2; // need pairs of uniforms

    for _ in 0..pairs {
        let u1: f64 = rng.random::<f64>();
        let u2: f64 = rng.random::<f64>();

        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;

        result.push(r * theta.cos());
        if result.len() < n {
            result.push(r * theta.sin());
        }
    }
    result
}

/// Generate anti-thetic pairs of Gaussian variates.
///
/// For each random variate z, also use -z. This variance reduction
/// technique ensures that for every upward path there is a corresponding
/// downward path, reducing the variance of the Monte Carlo estimator.
pub fn generate_antithetic_gaussians<R: Rng>(rng: &mut R, n: usize) -> (Vec<f64>, Vec<f64>) {
    let gaussians = generate_gaussians(rng, n);
    let antithetic: Vec<f64> = gaussians.iter().map(|&z| -z).collect();
    (gaussians, antithetic)
}

/// Standard normal cumulative distribution function (CDF).
///
/// Uses the Abramowitz & Stegun rational approximation (equation 26.2.17).
/// Maximum error: 7.5e-8.
pub fn normal_cdf(x: f64) -> f64 {
    if x >= 0.0 {
        normal_cdf_positive(x)
    } else {
        1.0 - normal_cdf_positive(-x)
    }
}

fn normal_cdf_positive(x: f64) -> f64 {
    const A1: f64 = 0.319381530;
    const A2: f64 = -0.356563782;
    const A3: f64 = 1.781477937;
    const A4: f64 = -1.821255978;
    const A5: f64 = 1.330274429;
    const RSQRT2PI: f64 = 0.398_942_280_401_432_7; // 1/sqrt(2*pi)

    let k = 1.0 / (1.0 + 0.2316419 * x);
    let k2 = k * k;
    let k3 = k2 * k;
    let k4 = k3 * k;
    let k5 = k4 * k;

    let poly = A1 * k + A2 * k2 + A3 * k3 + A4 * k4 + A5 * k5;
    let pdf = RSQRT2PI * (-0.5 * x * x).exp();

    1.0 - pdf * poly
}

/// Standard normal probability density function (PDF).
pub fn normal_pdf(x: f64) -> f64 {
    const RSQRT2PI: f64 = 0.398_942_280_401_432_7;
    RSQRT2PI * (-0.5 * x * x).exp()
}

/// Inverse normal CDF (quantile function).
///
/// Uses the Beasley-Springer-Moro algorithm.
pub fn normal_inv_cdf(p: f64) -> f64 {
    assert!((0.0..=1.0).contains(&p), "probability must be in [0, 1]");

    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }

    // Rational approximation for central region
    const A: [f64; 4] = [2.50662823884, -18.61500062529, 41.39119773534, -25.44106049637];
    const B: [f64; 4] = [-8.47351093090, 23.08336743743, -21.06224101826, 3.13082909833];
    const C: [f64; 9] = [
        0.3374754822726147, 0.9761690190917186, 0.1607979714918209,
        0.0276438810333863, 0.0038405729373609, 0.0003951896511919,
        0.0000321767881768, 0.0000002888167364, 0.0000003960315187,
    ];

    let y = p - 0.5;

    if y.abs() < 0.42 {
        let r = y * y;
        let num = y * (((A[3] * r + A[2]) * r + A[1]) * r + A[0]);
        let den = (((B[3] * r + B[2]) * r + B[1]) * r + B[0]) * r + 1.0;
        num / den
    } else {
        let r = if y < 0.0 { p } else { 1.0 - p };
        let s = (-r.ln()).ln();
        let mut t = C[0];
        for i in 1..9 {
            t += C[i] * s.powi(i as i32);
        }
        if y < 0.0 { -t } else { t }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_cdf_symmetry() {
        let eps = 1e-7;
        assert!((normal_cdf(0.0) - 0.5).abs() < eps);
        assert!((normal_cdf(1.96) - 0.975).abs() < 0.001);
        assert!((normal_cdf(-1.96) - 0.025).abs() < 0.001);
    }

    #[test]
    fn test_box_muller_count() {
        let mut rng = rand::rng();
        let v = box_muller(&mut rng, 1000);
        assert_eq!(v.len(), 1000);
    }

    #[test]
    fn test_antithetic_sum_zero() {
        let mut rng = rand::rng();
        let (pos, neg) = generate_antithetic_gaussians(&mut rng, 100);
        for (a, b) in pos.iter().zip(neg.iter()) {
            assert!((a + b).abs() < 1e-12);
        }
    }

    #[test]
    fn test_inv_cdf_roundtrip() {
        for &p in &[0.01, 0.1, 0.25, 0.5, 0.75, 0.9, 0.99] {
            let x = normal_inv_cdf(p);
            let p_back = normal_cdf(x);
            assert!((p - p_back).abs() < 0.001, "p={p}, x={x}, p_back={p_back}");
        }
    }
}
