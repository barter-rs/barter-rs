use crate::v2::{
    order::{OpenInFlight, Order, RequestCancel, RequestOpen},
    StateUpdater,
};
use derive_more::{Constructor, Display, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Todo:
/// *EXAMPLE IMPLEMENTATION ONLY, PLEASE DO NOT USE FOR ANYTHING OTHER THAN TESTING PURPOSES.*
pub mod default;

pub trait RiskManager<EngineState> {
    type Event;
    type State: for<'a> StateUpdater<&'a Self::Event> + Debug + Clone;

    fn check<InstrumentKey>(
        &self,
        engine_state: &EngineState,
        cancels: impl IntoIterator<Item = Order<InstrumentKey, RequestCancel>>,
        opens: impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>,
    ) -> (
        impl Iterator<Item = RiskApproved<Order<InstrumentKey, RequestCancel>>>,
        impl Iterator<Item = RiskApproved<Order<InstrumentKey, RequestOpen>>>,
        impl Iterator<Item = RiskRefused<Order<InstrumentKey, RequestCancel>>>,
        impl Iterator<Item = RiskRefused<Order<InstrumentKey, RequestOpen>>>,
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
        Order::from(&value.0)
    }
}

impl<InstrumentKey: Clone> From<&Order<InstrumentKey, RequestOpen>> for Order<InstrumentKey, OpenInFlight> {
    fn from(value: &Order<InstrumentKey, RequestOpen>) -> Self {
        let Order {
            instrument,
            cid,
            side,
            state: _,
        } = &value;

        Self {
            instrument: instrument.clone(),
            cid: *cid,
            side: *side,
            state: OpenInFlight,
        }
    }
}
