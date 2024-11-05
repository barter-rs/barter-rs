use super::SubscriptionKind;
use barter_macro::{DeSubKind, SerSubKind};
use serde::{Deserialize, Serialize};

/// Barter [`Subscription`](super::Subscription) [`SubscriptionKind`] that yields account
/// [`Event<T>`](crate::event::Event) events.
///
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, DeSubKind, SerSubKind)]
pub struct Accounts;

impl SubscriptionKind for Accounts {
    type Event = Account;

    fn as_str(&self) -> &'static str {
        "account"
    }
}

/// Normalised Barter [`Account`].
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct Account {}
