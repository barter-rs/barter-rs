//! Rust implementations of the C++ examples from
//! *The Concepts and Practice of Mathematical Finance* by Mark S. Joshi.
//!
//! # Modules
//!
//! - [`payoff`] — Polymorphic PayOff trait: Call, Put, Digital, DoubleDigital
//! - [`random`] — Gaussian random number generation (Box-Muller)
//! - [`monte_carlo`] — Simple and path-dependent Monte Carlo pricing
//! - [`black_scholes`] — Closed-form Black-Scholes pricing and Greeks
//! - [`parameters`] — Time-varying volatility and interest rate parameters
//! - [`trees`] — Binomial tree option pricing
//! - [`solvers`] — Newton-Raphson and bisection root-finding (implied volatility)
//! - [`statistics`] — Convergence table and statistics gathering
//! - [`path_dependent`] — Asian and barrier option pricing

pub mod payoff;
pub mod random;
pub mod monte_carlo;
pub mod black_scholes;
pub mod parameters;
pub mod trees;
pub mod solvers;
pub mod statistics;
pub mod path_dependent;
