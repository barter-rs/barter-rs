use crate::engine::Processor;
use barter_data::event::MarketEvent;
use barter_execution::{
    order::{Order, RequestCancel, RequestOpen},
    AccountEvent,
};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};
use barter_integration::Unrecoverable;
use derive_more::{Constructor, Display, From};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, hash::Hash, marker::PhantomData};

pub trait RiskManager<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    type State;

    fn check(
        &self,
        state: &Self::State,
        cancels: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        opens: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskApproved<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
        impl IntoIterator<Item = RiskRefused<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskRefused<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
    );
}

pub trait RiskManagerNew<ExchangeKey, AssetKey, InstrumentKey> {
    type State;

    fn check(
        &self,
        state: &Self::State,
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

/// Example [`RiskManager`] implementation that approves all order requests.
///
/// *EXAMPLE IMPLEMENTATION ONLY, PLEASE DO NOT USE FOR ANYTHING OTHER THAN TESTING PURPOSES.*
#[derive(Debug, Clone)]
pub struct DefaultRiskManager<State> {
    phantom: PhantomData<State>,
}

impl<State> Default for DefaultRiskManager<State> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<State, ExchangeKey, InstrumentKey> RiskManager<ExchangeKey, InstrumentKey>
    for DefaultRiskManager<State>
{
    type State = State;

    fn check(
        &self,
        _: &Self::State,
        cancels: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
        opens: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskApproved<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
        impl IntoIterator<Item = RiskRefused<Order<ExchangeKey, InstrumentKey, RequestCancel>>>,
        impl IntoIterator<Item = RiskRefused<Order<ExchangeKey, InstrumentKey, RequestOpen>>>,
    ) {
        (
            cancels.into_iter().map(RiskApproved::new),
            opens.into_iter().map(RiskApproved::new),
            std::iter::empty(),
            std::iter::empty(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct DefaultRiskManagerState;

impl<ExchangeKey, AssetKey, InstrumentKey>
    Processor<&AccountEvent<ExchangeKey, AssetKey, InstrumentKey>> for DefaultRiskManagerState
{
    type Output = ();
    fn process(&mut self, _: &AccountEvent<ExchangeKey, AssetKey, InstrumentKey>) -> Self::Output {}
}

impl<InstrumentKey, Kind> Processor<&MarketEvent<InstrumentKey, Kind>> for DefaultRiskManagerState {
    type Output = ();
    fn process(&mut self, _: &MarketEvent<InstrumentKey, Kind>) -> Self::Output {}
}
