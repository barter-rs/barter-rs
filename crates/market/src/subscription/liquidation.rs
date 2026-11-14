use super::SubscriptionKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use trading_bot_instrument::Side;

/// Trading Bot [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields [`Liquidation`]
/// [`MarketEvent<T>`](crate::event::MarketEvent) events.
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize,
)]
pub struct Liquidations;

impl SubscriptionKind for Liquidations {
    type Event = Liquidation;

    fn as_str(&self) -> &'static str {
        "liquidations"
    }
}

impl std::fmt::Display for Liquidations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Normalised Trading Bot [`Liquidation`] model.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Liquidation {
    pub side: Side,
    pub price: f64,
    pub quantity: f64,
    pub time: DateTime<Utc>,
}
