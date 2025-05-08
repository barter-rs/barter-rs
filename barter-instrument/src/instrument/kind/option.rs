use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::fmt::{Display, Formatter};

/// `OptionContract` specification containing all the information needed to fully identify an
/// option instrument.
///
/// # Type Parameters
/// * `AssetKey` - Type used to identify the settlement asset for the option contract.
///
/// # Fields
/// * `contract_size` - Multiplier that determines how many of the underlying asset the contract represents.
/// * `settlement_asset` - Asset used for settlement when the option is exercised.
/// * `kind` - Call (right to buy) or Put (right to sell).
/// * `exercise` - Exercise style (American, European, or Bermudan) defining when the option
///   can be exercised.
/// * `expiry` - The date and time when the option expires.
/// * `strike` - The price at which the option holder can buy (for calls) or sell (for puts)
///   the underlying asset upon exercise.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct OptionContract<AssetKey> {
    pub contract_size: Decimal,
    pub settlement_asset: AssetKey,
    pub kind: OptionKind,
    pub exercise: OptionExercise,
    #[cfg_attr(feature = "serde", serde(with = "chrono::serde::ts_milliseconds"))]
    pub expiry: DateTime<Utc>,
    pub strike: Decimal,
}
/// [`OptionContract`] kind - Put or Call.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OptionKind {
    #[cfg_attr(feature = "serde", serde(alias = "CALL", alias = "Call"))]
    Call,
    #[cfg_attr(feature = "serde", serde(alias = "PUT", alias = "Put"))]
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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OptionExercise {
    #[cfg_attr(feature = "serde", serde(alias = "AMERICAN", alias = "American"))]
    American,
    #[cfg_attr(feature = "serde", serde(alias = "BERMUDAN", alias = "Bermudan"))]
    Bermudan,
    #[cfg_attr(feature = "serde", serde(alias = "EUROPEAN", alias = "European"))]
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
