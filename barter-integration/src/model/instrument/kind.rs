use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Defines the type of [`Instrument`](super::Instrument) which is being traded on a
/// given `base_quote` market.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    Spot,
    Future(FutureContract),
    Perpetual,
    Option(OptionContract),
}

impl Default for InstrumentKind {
    fn default() -> Self {
        Self::Spot
    }
}

impl Display for InstrumentKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InstrumentKind::Spot => "spot".to_string(),
                InstrumentKind::Future(future) =>
                    format!("future_{}-UTC", future.expiry.date_naive()),
                InstrumentKind::Perpetual => "perpetual".to_string(),
                InstrumentKind::Option(option) => format!(
                    "option_{}_{}_{}-UTC_{}",
                    option.kind,
                    option.exercise,
                    option.expiry.date_naive(),
                    option.strike,
                ),
            }
        )
    }
}

/// Configuration of an [`InstrumentKind::Future`] contract.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
pub struct FutureContract {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub expiry: DateTime<Utc>,
}

/// Configuration of an [`InstrumentKind::Option`] contract.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
pub struct OptionContract {
    pub kind: OptionKind,
    pub exercise: OptionExercise,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub expiry: DateTime<Utc>,
    pub strike: Decimal,
}

/// [`OptionContract`] kind - Put or Call.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OptionKind {
    #[serde(alias = "CALL", alias = "Call")]
    Call,
    #[serde(alias = "PUT", alias = "Put")]
    Put,
}

impl Display for OptionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                OptionKind::Call => "call",
                OptionKind::Put => "put",
            }
        )
    }
}

/// [`OptionContract`] exercise style.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OptionExercise {
    #[serde(alias = "AMERICAN", alias = "American")]
    American,
    #[serde(alias = "BERMUDAN", alias = "Bermudan")]
    Bermudan,
    #[serde(alias = "EUROPEAN", alias = "European")]
    European,
}

impl Display for OptionExercise {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                OptionExercise::American => "american",
                OptionExercise::Bermudan => "bermudan",
                OptionExercise::European => "european",
            }
        )
    }
}
