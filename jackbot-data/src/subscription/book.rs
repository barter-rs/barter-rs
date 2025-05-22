use super::SubscriptionKind;
use crate::books::{Level, OrderBook, mid_price, volume_weighted_mid_price};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use jackbot_macro::{DeSubKind, SerSubKind};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Jackbot [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields [`OrderBookL1`]
/// market events.
///
/// Level 1 refers to the best non-aggregated bid and ask [`Level`] on each side of the
/// [`OrderBook`].

/// Jackbot [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields L2
/// [`OrderBookEvent`] market events
///
/// Level 2 refers to an [`OrderBook`] with orders at each price level aggregated.
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeSubKind, SerSubKind,
)]
pub struct OrderBooksL2;

impl SubscriptionKind for OrderBooksL2 {
    type Event = OrderBookEvent;
    fn as_str(&self) -> &'static str {
        "l2"
    }
}

impl std::fmt::Display for OrderBooksL2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Jackbot [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields
/// L3 [`OrderBookEvent`] market events.
///
/// Level 3 refers to the non-aggregated [`OrderBook`]. This is a direct replication of the exchange
/// [`OrderBook`].
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeSubKind, SerSubKind,
)]
pub struct OrderBooksL3;

impl SubscriptionKind for OrderBooksL3 {
    type Event = OrderBookEvent;

    fn as_str(&self) -> &'static str {
        "l3"
    }
}

impl std::fmt::Display for OrderBooksL3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum OrderBookEvent {
    Snapshot(OrderBook),
    Update(OrderBook),
}
