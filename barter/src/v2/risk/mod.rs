use crate::v2::{
    engine::state::EngineState,
    order::{Order, RequestCancel, RequestOpen},
};
use derive_more::{Constructor, Display, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Todo:
/// *EXAMPLE IMPLEMENTATION ONLY, PLEASE DO NOT USE FOR ANYTHING OTHER THAN TESTING PURPOSES.*
pub mod default;

pub trait RiskManager<InstrumentState, BalanceState, AssetKey, InstrumentKey> {
    type State;
    type StrategyState;

    fn check(
        &self,
        engine_state: &EngineState<
            InstrumentState,
            BalanceState,
            Self::StrategyState,
            Self::State,
            AssetKey,
            InstrumentKey,
        >,
        cancels: impl IntoIterator<Item = Order<InstrumentKey, RequestCancel>>,
        opens: impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<Order<InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskApproved<Order<InstrumentKey, RequestOpen>>>,
        impl IntoIterator<Item = RiskRefused<Order<InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskRefused<Order<InstrumentKey, RequestOpen>>>,
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
