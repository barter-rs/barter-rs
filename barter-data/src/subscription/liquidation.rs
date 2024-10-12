use super::SubscriptionKind;
use barter_integration::model::Side;
use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};

/// Barter [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields [`Liquidation`]
/// [`MarketEvent<T>`](crate::event::MarketEvent) events.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Default,
    Deserialize,
    Serialize,
    Display,
)]
pub struct Liquidations;

impl SubscriptionKind for Liquidations {
    type Event = Liquidation;

    fn as_str(&self) -> &'static str {
        "liquidations"
    }
}

/// Normalised Barter [`Liquidation`] model.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Liquidation {
    pub side: Side,
    pub price: f64,
    pub quantity: f64,
    pub time: DateTime<Utc>,
}
