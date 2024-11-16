use crate::v2::{
    engine::state::{asset::AssetStates, instrument::InstrumentStates},
    order::{Order, RequestCancel, RequestOpen},
};
use barter_integration::Unrecoverable;
use derive_more::{Constructor, Display, From};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, hash::Hash};

/// Todo:
/// *EXAMPLE IMPLEMENTATION ONLY, PLEASE DO NOT USE FOR ANYTHING OTHER THAN TESTING PURPOSES.*
pub mod default;

pub trait RiskManager<MarketState, ExchangeKey, AssetKey, InstrumentKey> {
    type State: Clone;

    fn check(
        &self,
        risk_state: &Self::State,
        asset_states: &AssetStates,
        instrument_states: &InstrumentStates<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
        cancels: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        opens: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskApproved<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
        impl IntoIterator<Item = RiskRefused<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskRefused<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
    );
}

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Deserialize,
    Serialize,
    Display,
    From,
    Constructor,
)]
pub struct RiskApproved<T>(pub T);

impl<T> RiskApproved<T> {
    pub fn into_item(self) -> T {
        self.0
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct RiskRefused<T, Reason = String> {
    pub item: T,
    pub reason: Reason,
}

impl<T, Reason> RiskRefused<T, Reason> {
    pub fn into_item(self) -> T {
        self.item
    }
}

impl<T, Reason> Unrecoverable for RiskRefused<T, Reason>
where
    Reason: Unrecoverable,
{
    fn is_unrecoverable(&self) -> bool {
        self.reason.is_unrecoverable()
    }
}
