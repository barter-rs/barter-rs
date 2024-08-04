use crate::v2::{
    order::{OpenInFlight, Order, RequestCancel, RequestOpen},
    TryUpdater,
};
use barter_data::instrument::InstrumentId;
use derive_more::{Constructor, Display, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Todo:
/// *EXAMPLE IMPLEMENTATION ONLY, PLEASE DO NOT USE FOR ANYTHING OTHER THAN TESTING PURPOSES.*
pub mod default;

pub trait RiskManager<EngineState> {
    type Event;
    type State: for<'a> TryUpdater<&'a Self::Event> + Debug + Clone;

    fn approve_orders(
        &self,
        engine_state: &EngineState,
        cancels: impl IntoIterator<Item = Order<InstrumentId, RequestCancel>>,
        opens: impl IntoIterator<Item = Order<InstrumentId, RequestOpen>>,
    ) -> (
        impl Iterator<Item = RiskApproved<Order<InstrumentId, RequestCancel>>>,
        impl Iterator<Item = RiskApproved<Order<InstrumentId, RequestOpen>>>,
        impl Iterator<Item = RiskRefused<Order<InstrumentId, RequestCancel>>>,
        impl Iterator<Item = RiskRefused<Order<InstrumentId, RequestOpen>>>,
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

impl<InstrumentKey: Clone> From<&RiskApproved<Order<InstrumentKey, RequestOpen>>>
    for Order<InstrumentKey, OpenInFlight>
{
    fn from(value: &RiskApproved<Order<InstrumentKey, RequestOpen>>) -> Self {
        let Order {
            instrument,
            cid,
            side,
            state: _,
        } = &value.0;

        Self {
            instrument: instrument.clone(),
            cid: *cid,
            side: *side,
            state: OpenInFlight,
        }
    }
}
