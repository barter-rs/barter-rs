use super::SubscriptionKind;
use crate::books::{mid_price, volume_weighted_mid_price, Level, OrderBook};
use barter_macro::{DeSubKind, SerSubKind};
use chrono::{DateTime, Utc};
use derive_more::Display;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Barter [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields level 1 [`OrderBook`]
/// [`MarketEvent<T>`](MarketEvent) events.
///
/// Level 1 refers to the best non-aggregated bid and ask [`Level`] on each side of the
/// [`OrderBook`].
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
    DeSubKind,
    SerSubKind,
    Display,
)]
pub struct OrderBooksL1;

impl SubscriptionKind for OrderBooksL1 {
    type Event = OrderBookL1;
    fn as_str(&self) -> &'static str {
        "order_books_l1"
    }
}

/// Normalised Barter [`OrderBookL1`] snapshot containing the latest best bid and ask.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OrderBookL1 {
    pub last_update_time: DateTime<Utc>,
    pub best_bid: Level,
    pub best_ask: Level,
}

impl OrderBookL1 {
    /// Calculate the mid-price by taking the average of the best bid and ask prices.
    ///
    /// See Docs: <https://www.quantstart.com/articles/high-frequency-trading-ii-limit-order-book>
    pub fn mid_price(&self) -> Decimal {
        mid_price(self.best_bid.price, self.best_ask.price)
    }

    /// Calculate the volume weighted mid-price (micro-price), weighing the best bid and ask prices
    /// with their associated amount.
    ///
    /// See Docs: <https://www.quantstart.com/articles/high-frequency-trading-ii-limit-order-book>
    pub fn volume_weighed_mid_price(&self) -> Decimal {
        volume_weighted_mid_price(self.best_bid, self.best_ask)
    }
}

/// Barter [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields level 2 [`OrderBook`]
/// [`MarketEvent<T>`](MarketEvent) events.
///
/// Level 2 refers to the [`OrderBook`] aggregated by price.
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
    DeSubKind,
    SerSubKind,
    Display,
)]
pub struct OrderBooksL2;

impl SubscriptionKind for OrderBooksL2 {
    type Event = OrderBookEvent;
    fn as_str(&self) -> &'static str {
        "order_books_l2"
    }
}

/// Barter [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields level 3 [`OrderBook`]
/// [`MarketEvent<T>`](MarketEvent) events.
///
/// Level 3 refers to the non-aggregated [`OrderBook`]. This is a direct replication of the exchange
/// [`OrderBook`].
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
    DeSubKind,
    SerSubKind,
    Display,
)]
pub struct OrderBooksL3;

impl SubscriptionKind for OrderBooksL3 {
    type Event = OrderBookEvent;

    fn as_str(&self) -> &'static str {
        "order_books_l2"
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum OrderBookEvent {
    Snapshot(OrderBook),
    Update(OrderBook),
}
