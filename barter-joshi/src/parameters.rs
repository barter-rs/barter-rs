//! Time-varying parameters for volatility and interest rates.
//!
//! Implements the Parameters class hierarchy from Joshi Chapter 7.6.
//! In Joshi's C++, this is handled via virtual functions and a bridge pattern.
//! In Rust, we use an enum or trait-based approach.

use serde::{Deserialize, Serialize};

/// A parameter that can be constant or time-varying.
///
/// Provides both the value at a point in time and the integral over a time range.
pub trait Parameter: Send + Sync {
    /// Value of the parameter at time t.
    fn value(&self, t: f64) -> f64;

    /// Integral of the parameter from time t1 to t2.
    fn integral(&self, t1: f64, t2: f64) -> f64;

    /// Mean value over the interval [t1, t2].
    fn mean(&self, t1: f64, t2: f64) -> f64 {
        if (t2 - t1).abs() < 1e-15 {
            return self.value(t1);
        }
        self.integral(t1, t2) / (t2 - t1)
    }

    /// Root-mean-square value over [t1, t2] (useful for volatility).
    fn rms(&self, t1: f64, t2: f64) -> f64 {
        self.mean(t1, t2).sqrt()
    }
}

/// Integral of the square of a parameter from t1 to t2.
pub trait SquareIntegral: Parameter {
    fn square_integral(&self, t1: f64, t2: f64) -> f64;

    /// Root-mean-square: sqrt(1/(t2-t1) * integral(f^2 dt)).
    fn root_mean_square(&self, t1: f64, t2: f64) -> f64 {
        if (t2 - t1).abs() < 1e-15 {
            return self.value(t1);
        }
        (self.square_integral(t1, t2) / (t2 - t1)).sqrt()
    }
}

/// Constant parameter: f(t) = c for all t.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Constant {
    pub value: f64,
}

impl Constant {
    pub fn new(value: f64) -> Self {
        Self { value }
    }
}

impl Parameter for Constant {
    fn value(&self, _t: f64) -> f64 {
        self.value
    }

    fn integral(&self, t1: f64, t2: f64) -> f64 {
        self.value * (t2 - t1)
    }
}

impl SquareIntegral for Constant {
    fn square_integral(&self, t1: f64, t2: f64) -> f64 {
        self.value * self.value * (t2 - t1)
    }
}

/// Piecewise-constant parameter: f(t) = values[i] for times[i] <= t < times[i+1].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiecewiseConstant {
    /// Boundary times (must be sorted, length n+1 for n pieces).
    pub times: Vec<f64>,
    /// Values for each piece (length n).
    pub values: Vec<f64>,
}

impl PiecewiseConstant {
    pub fn new(times: Vec<f64>, values: Vec<f64>) -> Self {
        assert_eq!(times.len(), values.len() + 1, "times.len() must be values.len() + 1");
        assert!(times.windows(2).all(|w| w[0] < w[1]), "times must be strictly increasing");
        Self { times, values }
    }
}

impl Parameter for PiecewiseConstant {
    fn value(&self, t: f64) -> f64 {
        for i in 0..self.values.len() {
            if t < self.times[i + 1] {
                return self.values[i];
            }
        }
        *self.values.last().unwrap_or(&0.0)
    }

    fn integral(&self, t1: f64, t2: f64) -> f64 {
        if t1 >= t2 {
            return 0.0;
        }
        let mut result = 0.0;
        for i in 0..self.values.len() {
            let lo = self.times[i].max(t1);
            let hi = self.times[i + 1].min(t2);
            if lo < hi {
                result += self.values[i] * (hi - lo);
            }
        }
        result
    }
}

impl SquareIntegral for PiecewiseConstant {
    fn square_integral(&self, t1: f64, t2: f64) -> f64 {
        if t1 >= t2 {
            return 0.0;
        }
        let mut result = 0.0;
        for i in 0..self.values.len() {
            let lo = self.times[i].max(t1);
            let hi = self.times[i + 1].min(t2);
            if lo < hi {
                result += self.values[i] * self.values[i] * (hi - lo);
            }
        }
        result
    }
}

/// Linearly interpolated parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Linear {
    /// Time points.
    pub times: Vec<f64>,
    /// Values at each time point.
    pub values: Vec<f64>,
}

impl Linear {
    pub fn new(times: Vec<f64>, values: Vec<f64>) -> Self {
        assert_eq!(times.len(), values.len());
        assert!(times.windows(2).all(|w| w[0] < w[1]));
        Self { times, values }
    }
}

impl Parameter for Linear {
    fn value(&self, t: f64) -> f64 {
        if t <= self.times[0] {
            return self.values[0];
        }
        if t >= *self.times.last().unwrap() {
            return *self.values.last().unwrap();
        }
        for i in 0..self.times.len() - 1 {
            if t >= self.times[i] && t <= self.times[i + 1] {
                let frac = (t - self.times[i]) / (self.times[i + 1] - self.times[i]);
                return self.values[i] + frac * (self.values[i + 1] - self.values[i]);
            }
        }
        *self.values.last().unwrap()
    }

    fn integral(&self, t1: f64, t2: f64) -> f64 {
        // Trapezoidal approximation with 1000 steps
        let n = 1000;
        let dt = (t2 - t1) / n as f64;
        let mut sum = 0.5 * (self.value(t1) + self.value(t2));
        for i in 1..n {
            sum += self.value(t1 + i as f64 * dt);
        }
        sum * dt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_parameter() {
        let c = Constant::new(0.2);
        assert_eq!(c.value(0.5), 0.2);
        assert!((c.integral(0.0, 1.0) - 0.2).abs() < 1e-10);
        assert!((c.mean(0.0, 1.0) - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_piecewise_constant() {
        // vol = 0.2 for [0, 0.5), vol = 0.3 for [0.5, 1.0)
        let pc = PiecewiseConstant::new(vec![0.0, 0.5, 1.0], vec![0.2, 0.3]);
        assert_eq!(pc.value(0.25), 0.2);
        assert_eq!(pc.value(0.75), 0.3);
        let integral = pc.integral(0.0, 1.0);
        assert!((integral - 0.25).abs() < 1e-10, "integral: {integral}");
    }

    #[test]
    fn test_linear_interpolation() {
        let lin = Linear::new(vec![0.0, 1.0], vec![0.1, 0.3]);
        assert!((lin.value(0.5) - 0.2).abs() < 1e-10);
        assert!((lin.value(0.0) - 0.1).abs() < 1e-10);
        assert!((lin.value(1.0) - 0.3).abs() < 1e-10);
    }
}
