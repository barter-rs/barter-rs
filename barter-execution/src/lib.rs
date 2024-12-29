#![forbid(unsafe_code)]
#![warn(
    unused,
    clippy::cognitive_complexity,
    unused_crate_dependencies,
    unused_extern_crates,
    clippy::unused_self,
    clippy::useless_let_if_seq,
    missing_debug_implementations,
    rust_2018_idioms,
    rust_2024_compatibility
)]
#![allow(clippy::type_complexity, clippy::too_many_arguments, type_alias_bounds)]

//! # Barter-Execution
//! High-performance and normalised trading interface capable of executing across many financial
//! venues. Also provides a feature rich simulated execution to assist with backtesting
//! and dry-trading. Communicate with an execution by initialising it's associated
//! `ExecutionClient` instance.
//! **It is:**
//! * **Easy**: ExecutionClient trait provides a unified and simple language for interacting with
//!   exchanges.
//! * **Normalised**: Allow your strategy to communicate with every real or simulated execution
//!   using the same interface.
//! * **Extensible**: Barter-Execution is highly extensible, making it easy to contribute by adding
//!   new execution integrations!
//!
//! See `README.md` for more information and examples.

use crate::{
    balance::AssetBalance,
    error::ClientError,
    order::{Cancelled, ExchangeOrderState, Open, Order},
    trade::Trade,
};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex, QuoteAsset},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
};
use barter_integration::snapshot::Snapshot;
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};

pub mod balance;
pub mod client;
pub mod error;
pub mod exchange;
pub mod indexer;
pub mod map;
pub mod order;
pub mod trade;

pub type FnvIndexMap<K, V> = indexmap::IndexMap<K, V, fnv::FnvBuildHasher>;
pub type FnvIndexSet<T> = indexmap::IndexSet<T, fnv::FnvBuildHasher>;

/// Convenient type alias for an [`AccountEvent`] keyed with [`ExchangeId`],
/// [`AssetNameExchange`], and [`InstrumentNameExchange`].
pub type UnindexedAccountEvent =
    AccountEvent<ExchangeId, AssetNameExchange, InstrumentNameExchange>;

/// Convenient type alias for an [`AccountSnapshot`] keyed with [`ExchangeId`],
/// [`AssetNameExchange`], and [`InstrumentNameExchange`].
pub type UnindexedAccountSnapshot =
    AccountSnapshot<ExchangeId, AssetNameExchange, InstrumentNameExchange>;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AccountEvent<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
> {
    pub exchange: ExchangeKey,
    pub kind: AccountEventKind<ExchangeKey, AssetKey, InstrumentKey>,
}

impl<ExchangeKey, AssetKey, InstrumentKey> AccountEvent<ExchangeKey, AssetKey, InstrumentKey> {
    pub fn new<K>(exchange: ExchangeKey, kind: K) -> Self
    where
        K: Into<AccountEventKind<ExchangeKey, AssetKey, InstrumentKey>>,
    {
        Self {
            exchange,
            kind: kind.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From)]
pub enum AccountEventKind<ExchangeKey, AssetKey, InstrumentKey> {
    /// Full [`AccountSnapshot`] - replaces all existing state.
    Snapshot(AccountSnapshot<ExchangeKey, AssetKey, InstrumentKey>),

    /// Single [`AssetBalance`] snapshot - replaces existing balance state.
    BalanceSnapshot(Snapshot<AssetBalance<AssetKey>>),

    /// Single [`Order<ExchangeKey, InstrumentKey, Open>`] snapshot - replaces existing order state.
    OrderSnapshot(Snapshot<Order<ExchangeKey, InstrumentKey, ExchangeOrderState>>),

    /// Response to an [`Order<ExchangeKey, InstrumentKey, RequestOpen>`].
    OrderOpened(
        Order<ExchangeKey, InstrumentKey, Result<Open, ClientError<AssetKey, InstrumentKey>>>,
    ),
    /// Response to an [`Order<ExchangeKey, InstrumentKey, RequestCancel>`].
    OrderCancelled(
        Order<ExchangeKey, InstrumentKey, Result<Cancelled, ClientError<AssetKey, InstrumentKey>>>,
    ),

    /// [`Order<ExchangeKey, InstrumentKey, Open>`] partial or full fill.
    Trade(Trade<QuoteAsset, InstrumentKey>),
}

impl<ExchangeKey, AssetKey, InstrumentKey> AccountEvent<ExchangeKey, AssetKey, InstrumentKey>
where
    AssetKey: Eq,
    InstrumentKey: Eq,
{
    pub fn snapshot(self) -> Option<AccountSnapshot<ExchangeKey, AssetKey, InstrumentKey>> {
        if let AccountEventKind::Snapshot(snapshot) = self.kind {
            Some(snapshot)
        } else {
            None
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, Constructor,
)]
pub struct AccountSnapshot<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
> {
    pub exchange: ExchangeKey,
    pub balances: Vec<AssetBalance<AssetKey>>,
    pub instruments: Vec<InstrumentAccountSnapshot<ExchangeKey, InstrumentKey>>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, Constructor,
)]
pub struct InstrumentAccountSnapshot<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    pub instrument: InstrumentKey,
    pub orders: Vec<Order<ExchangeKey, InstrumentKey, ExchangeOrderState>>,
}

impl<ExchangeKey, AssetKey, InstrumentKey> AccountSnapshot<ExchangeKey, AssetKey, InstrumentKey> {
    pub fn assets(&self) -> impl Iterator<Item = &AssetKey> {
        self.balances.iter().map(|balance| &balance.asset)
    }

    pub fn instruments(&self) -> impl Iterator<Item = &InstrumentKey> {
        self.instruments.iter().map(|snapshot| &snapshot.instrument)
    }
}
