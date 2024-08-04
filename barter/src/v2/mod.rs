use crate::v2::{
    engine::command::Command,
    execution::{AccountEvent, AccountEventKind},
    instrument::asset::AssetId,
};
use barter_data::{event::MarketEvent, instrument::InstrumentId};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod balance;
pub mod channel;
pub mod engine;
pub mod execution;
pub mod instrument;
pub mod market_data;
pub mod order;
pub mod position;
pub mod risk;
pub mod strategy;
pub mod trade;

pub trait TryUpdater<Event> {
    type Error: Debug;
    fn try_update(&mut self, event: Event) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From)]
pub enum EngineEvent<AssetKey = AssetId, InstrumentKey = InstrumentId> {
    Command(Command),
    Account(AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>),
    Market(MarketEvent<InstrumentKey>),
}

#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Deserialize,
    Serialize,
    Constructor,
    From,
)]
pub struct Snapshot<T>(pub T);

impl<T> Snapshot<T> {
    pub const fn as_ref(&self) -> Snapshot<&T> {
        let Snapshot(x) = self;
        Snapshot(x)
    }
}
