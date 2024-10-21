use crate::instrument::kind::{future::FutureContract, option::OptionContract};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub mod future;
pub mod option;
pub mod perpetual;
pub mod spot;

/// Defines the type of [`Instrument`](Instrument) which is being traded on a
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
