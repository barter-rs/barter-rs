use crate::instrument::kind::option::{OptionExercise, OptionKind};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::fmt::{Display, Formatter};

/// Defines the type of [`MarketDataInstrument`](super::MarketDataInstrument) which is being
/// traded on a given `base_quote` market.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum MarketDataInstrumentKind {
    Spot,
    Perpetual,
    Future(MarketDataFutureContract),
    Option(MarketDataOptionContract),
}

impl Default for MarketDataInstrumentKind {
    fn default() -> Self {
        Self::Spot
    }
}

impl Display for MarketDataInstrumentKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MarketDataInstrumentKind::Spot => "spot".to_string(),
                MarketDataInstrumentKind::Perpetual => "perpetual".to_string(),
                MarketDataInstrumentKind::Future(contract) =>
                    format!("future_{}-UTC", contract.expiry.date_naive()),
                MarketDataInstrumentKind::Option(contract) => format!(
                    "option_{}_{}_{}-UTC_{}",
                    contract.kind,
                    contract.exercise,
                    contract.expiry.date_naive(),
                    contract.strike,
                ),
            }
        )
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MarketDataFutureContract {
    #[cfg_attr(feature = "serde", serde(with = "chrono::serde::ts_milliseconds"))]
    pub expiry: DateTime<Utc>,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MarketDataOptionContract {
    pub kind: OptionKind,
    pub exercise: OptionExercise,
    #[cfg_attr(feature = "serde", serde(with = "chrono::serde::ts_milliseconds"))]
    pub expiry: DateTime<Utc>,
    pub strike: Decimal,
}
