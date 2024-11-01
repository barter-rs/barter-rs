use crate::instrument::kind::{future::FutureContract, option::OptionContract};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Defines the type of [`MarketDataInstrument`](super::MarketDataInstrument) which is being
/// traded on a given `base_quote` market.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketDataInstrumentKind {
    Spot,
    Future(FutureContract),
    Perpetual,
    Option(OptionContract),
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
                MarketDataInstrumentKind::Future(future) =>
                    format!("future_{}-UTC", future.expiry.date_naive()),
                MarketDataInstrumentKind::Perpetual => "perpetual".to_string(),
                MarketDataInstrumentKind::Option(option) => format!(
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
