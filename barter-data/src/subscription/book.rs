use super::SubscriptionKind;
use crate::books::{Level, OrderBook, mid_price, volume_weighted_mid_price};
use barter_macro::{DeSubKind, SerSubKind};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Barter [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields [`OrderBookL1`]
/// market events.
///
/// Level 1 refers to the best non-aggregated bid and ask [`Level`] on each side of the
/// [`OrderBook`].
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeSubKind, SerSubKind,
)]
pub struct OrderBooksL1;

impl SubscriptionKind for OrderBooksL1 {
    type Event = OrderBookL1;
    fn as_str(&self) -> &'static str {
        "l1"
    }
}

impl std::fmt::Display for OrderBooksL1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Normalised Barter [`OrderBookL1`] snapshot containing the latest best bid and ask.
#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize, Constructor,
)]
pub struct OrderBookL1 {
    pub last_update_time: DateTime<Utc>,
    pub best_bid: Option<Level>,
    pub best_ask: Option<Level>,
}

impl OrderBookL1 {
    /// Calculate the mid-price by taking the average of the best bid and ask prices.
    ///
    /// See Docs: <https://www.quantstart.com/articles/high-frequency-trading-ii-limit-order-book>
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_ask, self.best_bid) {
            (Some(best_ask), Some(best_bid)) => Some(mid_price(best_bid.price, best_ask.price)),
            _ => None,
        }
    }

    /// Calculate the volume weighted mid-price (micro-price), weighing the best bid and ask prices
    /// with their associated amount.
    ///
    /// See Docs: <https://www.quantstart.com/articles/high-frequency-trading-ii-limit-order-book>
    pub fn volume_weighed_mid_price(&self) -> Option<Decimal> {
        match (self.best_ask, self.best_bid) {
            (Some(best_ask), Some(best_bid)) => Some(volume_weighted_mid_price(best_bid, best_ask)),
            _ => None,
        }
    }
}

/// Barter [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields L2
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

/// Barter [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields
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
