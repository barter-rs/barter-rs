use crate::v2::{
    engine::{command::Command, state::trading::TradingState},
    execution::{manager::AccountStreamEvent, AccountEvent},
};
use barter_data::{event::MarketEvent, streams::consumer::MarketStreamEvent};
use barter_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod balance;
pub mod engine;
pub mod error;
pub mod execution;
pub mod instrument;
pub mod order;
pub mod position;
pub mod risk;
pub mod strategy;
pub mod trade;

// Todo: Must Have:
//  - Utility to re-create state from Audit snapshot + updates w/ interactive mode
//    (backward would require Vec<State> to be created on .next()) (add compression using file system)
//  - All state update implementations
//  - Add tests for all Managers
//  - Engine functionality can be injected, on_shutdown, on_state_update_error, on_disconnect, etc.
//    '--> currently we are not reacting to "disconnected"

// Todo: Nice To Have:
//  - Sequenced log stream that can enrich logs w/ additional context eg/ InstrumentName
//  - Extract methods from impl OrderManager for Orders (eg/ update_from_snapshot covers all bases)
//    '--> also ensure duplication is removed from update_from_open & update_from_cancel
//  - Setup some way to get "diffs" for eg/ should Orders.update_from_order_snapshot return a diff?

// Todo: Nice To Have: OrderManager:
//  - OrderManager update_from_open & update_from_cancel may want to return "in flight failed due to X api reason"
//    '--> eg/ find logic associated with "OrderManager received ExecutionError for Order<InFlight>"
//  - Possible we want a 5m window buffer for "strange order updates" to handle out of orders
//    '--> eg/ adding InFlight, receiving Cancelled, the receiving Open -> ghost orders

pub type IndexedEngineEvent<MarketKind> =
    EngineEvent<MarketKind, ExchangeIndex, AssetIndex, InstrumentIndex>;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From)]
pub enum EngineEvent<MarketKind, ExchangeKey, AssetKey, InstrumentKey> {
    Shutdown,
    TradingStateUpdate(TradingState),
    Account(AccountStreamEvent<ExchangeKey, AssetKey, InstrumentKey>),
    Market(MarketStreamEvent<InstrumentKey, MarketKind>),
    Command(Command<ExchangeKey, InstrumentKey>),
}

impl<MarketKind, ExchangeKey, AssetKey, InstrumentKey>
    From<AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>
    for EngineEvent<MarketKind, ExchangeKey, AssetKey, InstrumentKey>
{
    fn from(value: AccountEvent<ExchangeKey, AssetKey, InstrumentKey>) -> Self {
        Self::Account(AccountStreamEvent::Item(value))
    }
}

impl<MarketKind, ExchangeKey, AssetKey, InstrumentKey> From<MarketEvent<InstrumentKey, MarketKind>>
    for EngineEvent<MarketKind, ExchangeKey, AssetKey, InstrumentKey>
{
    fn from(value: MarketEvent<InstrumentKey, MarketKind>) -> Self {
        Self::Market(MarketStreamEvent::Item(value))
    }
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
    pub fn as_ref(&self) -> Snapshot<&T> {
        let Self(item) = self;
        Snapshot(item)
    }
}
