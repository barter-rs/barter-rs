//! Option payoff functions.
//!
//! Implements the PayOff class hierarchy from Joshi Chapter 7.1–7.3.
//! Uses Rust traits instead of C++ virtual inheritance.

use serde::{Deserialize, Serialize};

/// A payoff function that maps a spot price at expiry to an option value.
///
/// This is the Rust equivalent of Joshi's abstract `PayOff` base class.
pub trait PayOff: Send + Sync {
    /// Compute the payoff given the final spot price.
    fn payoff(&self, spot: f64) -> f64;
}

/// European call option payoff: max(spot - strike, 0).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Call {
    pub strike: f64,
}

impl Call {
    pub fn new(strike: f64) -> Self {
        Self { strike }
    }
}

impl PayOff for Call {
    fn payoff(&self, spot: f64) -> f64 {
        (spot - self.strike).max(0.0)
    }
}

/// European put option payoff: max(strike - spot, 0).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Put {
    pub strike: f64,
}

impl Put {
    pub fn new(strike: f64) -> Self {
        Self { strike }
    }
}

impl PayOff for Put {
    fn payoff(&self, spot: f64) -> f64 {
        (self.strike - spot).max(0.0)
    }
}

/// Digital (binary) call: pays 1 if spot > strike, else 0.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DigitalCall {
    pub strike: f64,
}

impl DigitalCall {
    pub fn new(strike: f64) -> Self {
        Self { strike }
    }
}

impl PayOff for DigitalCall {
    fn payoff(&self, spot: f64) -> f64 {
        if spot > self.strike { 1.0 } else { 0.0 }
    }
}

/// Digital (binary) put: pays 1 if spot < strike, else 0.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DigitalPut {
    pub strike: f64,
}

impl DigitalPut {
    pub fn new(strike: f64) -> Self {
        Self { strike }
    }
}

impl PayOff for DigitalPut {
    fn payoff(&self, spot: f64) -> f64 {
        if spot < self.strike { 1.0 } else { 0.0 }
    }
}

/// Double digital: pays 1 if lower_strike < spot < upper_strike, else 0.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DoubleDigital {
    pub lower_strike: f64,
    pub upper_strike: f64,
}

impl DoubleDigital {
    pub fn new(lower_strike: f64, upper_strike: f64) -> Self {
        assert!(lower_strike < upper_strike, "lower_strike must be < upper_strike");
        Self { lower_strike, upper_strike }
    }
}

impl PayOff for DoubleDigital {
    fn payoff(&self, spot: f64) -> f64 {
        if spot > self.lower_strike && spot < self.upper_strike {
            1.0
        } else {
            0.0
        }
    }
}

/// Power option payoff: max(spot - strike, 0)^power.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PowerCall {
    pub strike: f64,
    pub power: f64,
}

impl PowerCall {
    pub fn new(strike: f64, power: f64) -> Self {
        Self { strike, power }
    }
}

impl PayOff for PowerCall {
    fn payoff(&self, spot: f64) -> f64 {
        let intrinsic = (spot - self.strike).max(0.0);
        intrinsic.powf(self.power)
    }
}

/// Straddle payoff: |spot - strike| (equivalent to call + put at same strike).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Straddle {
    pub strike: f64,
}

impl Straddle {
    pub fn new(strike: f64) -> Self {
        Self { strike }
    }
}

impl PayOff for Straddle {
    fn payoff(&self, spot: f64) -> f64 {
        (spot - self.strike).abs()
    }
}

/// Boxed payoff for dynamic dispatch (Joshi's PayOffBridge/Wrapper pattern).
impl PayOff for Box<dyn PayOff> {
    fn payoff(&self, spot: f64) -> f64 {
        (**self).payoff(spot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_payoff() {
        let call = Call::new(100.0);
        assert_eq!(call.payoff(110.0), 10.0);
        assert_eq!(call.payoff(100.0), 0.0);
        assert_eq!(call.payoff(90.0), 0.0);
    }

    #[test]
    fn test_put_payoff() {
        let put = Put::new(100.0);
        assert_eq!(put.payoff(90.0), 10.0);
        assert_eq!(put.payoff(100.0), 0.0);
        assert_eq!(put.payoff(110.0), 0.0);
    }

    #[test]
    fn test_digital_call() {
        let dc = DigitalCall::new(100.0);
        assert_eq!(dc.payoff(101.0), 1.0);
        assert_eq!(dc.payoff(100.0), 0.0);
        assert_eq!(dc.payoff(99.0), 0.0);
    }

    #[test]
    fn test_digital_put() {
        let dp = DigitalPut::new(100.0);
        assert_eq!(dp.payoff(99.0), 1.0);
        assert_eq!(dp.payoff(100.0), 0.0);
        assert_eq!(dp.payoff(101.0), 0.0);
    }

    #[test]
    fn test_double_digital() {
        let dd = DoubleDigital::new(90.0, 110.0);
        assert_eq!(dd.payoff(100.0), 1.0);
        assert_eq!(dd.payoff(85.0), 0.0);
        assert_eq!(dd.payoff(115.0), 0.0);
        assert_eq!(dd.payoff(90.0), 0.0);  // boundary: not strictly inside
    }

    #[test]
    fn test_straddle() {
        let s = Straddle::new(100.0);
        assert_eq!(s.payoff(110.0), 10.0);
        assert_eq!(s.payoff(90.0), 10.0);
        assert_eq!(s.payoff(100.0), 0.0);
    }

    #[test]
    fn test_dynamic_dispatch() {
        let payoffs: Vec<Box<dyn PayOff>> = vec![
            Box::new(Call::new(100.0)),
            Box::new(Put::new(100.0)),
            Box::new(DigitalCall::new(100.0)),
        ];
        let spot = 105.0;
        let results: Vec<f64> = payoffs.iter().map(|p| p.payoff(spot)).collect();
        assert_eq!(results, vec![5.0, 0.0, 1.0]);
    }
}
