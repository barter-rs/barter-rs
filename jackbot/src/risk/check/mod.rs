use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use thiserror::Error;

/// Utilities to assist with RiskManager checks.
///
/// For example, calculating notional values, price differences, etc.
pub mod util;

/// General interface for implementing simple RiskManager checks.
///
/// See [`CheckHigherThan`] for a simple example.
///
/// # Associated Types
/// * `Input` - The type of data being validated (e.g., `Decimal` for price checks)
/// * `Error` - The error type returned when validation fails
pub trait RiskCheck {
    type Input;
    type Error;

    /// Returns the name of the risk check.
    fn name() -> &'static str;

    /// Performs the risk check on the provided `Input`.
    fn check(&self, input: &Self::Input) -> Result<(), Self::Error>;
}

/// General risk check that validates if an input value exceeds an upper limit.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct CheckHigherThan<T> {
    /// The upper limit value; check passes if input is <= limit.
    pub limit: T,
}

impl<T> RiskCheck for CheckHigherThan<T>
where
    T: Clone + PartialOrd,
{
    type Input = T;
    type Error = CheckFailHigherThan<T>;

    fn name() -> &'static str {
        "CheckHigherThan"
    }

    fn check(&self, input: &Self::Input) -> Result<(), Self::Error> {
        if *input <= self.limit {
            Ok(())
        } else {
            Err(CheckFailHigherThan {
                limit: self.limit.clone(),
                input: input.clone(),
            })
        }
    }
}

/// Error returned when a [`CheckHigherThan`] validation fails.
#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize, Constructor, Error,
)]
#[error("CheckHigherThanFailed: input {input} > limit {limit}")]
pub struct CheckFailHigherThan<T> {
    /// The limit value that was exceeded.
    pub limit: T,

    /// The input value that caused the check to fail.
    pub input: T,
}

/// Risk check ensuring the potential loss of an order does not exceed a limit.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct CheckLossLimit {
    /// Maximum allowed loss.
    pub limit: Decimal,
}

/// Error returned when a [`CheckLossLimit`] validation fails.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize, Constructor, Error)]
#[error("CheckLossLimitFailed: loss {loss} > limit {limit}")]
pub struct CheckFailLossLimit {
    /// Configured loss limit.
    pub limit: Decimal,
    /// Calculated potential loss of the order.
    pub loss: Decimal,
}

impl RiskCheck for CheckLossLimit {
    type Input = Decimal;
    type Error = CheckFailLossLimit;

    fn name() -> &'static str {
        "CheckLossLimit"
    }

    fn check(&self, input: &Self::Input) -> Result<(), Self::Error> {
        if *input <= self.limit {
            Ok(())
        } else {
            Err(CheckFailLossLimit {
                limit: self.limit,
                loss: *input,
            })
        }
    }
}
