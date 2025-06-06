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
//! Stream private account data from financial venues, and execute (live or mock) orders. Also provides
//! a feature rich MockExchange and MockExecutionClient to assist with backtesting and paper-trading.
//!
//! **It is:**
//! * **Easy**: ExecutionClient trait provides a unified and simple language for interacting with exchanges.
//! * **Normalised**: Allow your strategy to communicate with every real or MockExchange using the same interface.
//! * **Extensible**: Barter-Execution is highly extensible, making it easy to contribute by adding new exchange integrations!
//!
//! See `README.md` for more information and examples.

use crate::{
    balance::AssetBalance,
    order::{Order, OrderSnapshot, request::OrderResponseCancel},
    trade::Trade,
};
use barter_instrument::{
    asset::{AssetIndex, QuoteAsset, name::AssetNameExchange},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{InstrumentIndex, name::InstrumentNameExchange},
};
use barter_integration::snapshot::Snapshot;
use chrono::{DateTime, Utc};
use derive_more::{Constructor, From};
use order::state::OrderState;
use serde::{Deserialize, Serialize};

pub mod balance;
pub mod client;
pub mod error;
pub mod exchange;
pub mod indexer;
pub mod map;
pub mod order;
pub mod trade;

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

    /// Single [`Order`] snapshot - used to upsert existing order state if it's more recent.
    ///
    /// This variant covers general order updates, and open order responses.
    OrderSnapshot(Snapshot<Order<ExchangeKey, InstrumentKey, OrderState<AssetKey, InstrumentKey>>>),

    /// Response to an [`OrderRequestCancel<ExchangeKey, InstrumentKey>`](order::request::OrderRequestOpen).
    OrderCancelled(OrderResponseCancel<ExchangeKey, AssetKey, InstrumentKey>),

    /// [`Order<ExchangeKey, InstrumentKey, Open>`] partial or full-fill.
    Trade(Trade<QuoteAsset, InstrumentKey>),
}

impl<ExchangeKey, AssetKey, InstrumentKey> AccountEvent<ExchangeKey, AssetKey, InstrumentKey>
where
    AssetKey: Eq,
    InstrumentKey: Eq,
{
    pub fn snapshot(self) -> Option<AccountSnapshot<ExchangeKey, AssetKey, InstrumentKey>> {
        match self.kind {
            AccountEventKind::Snapshot(snapshot) => Some(snapshot),
            _ => None,
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
    pub instruments: Vec<InstrumentAccountSnapshot<ExchangeKey, AssetKey, InstrumentKey>>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, Constructor,
)]
pub struct InstrumentAccountSnapshot<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
> {
    pub instrument: InstrumentKey,
    #[serde(default = "Vec::new")]
    pub orders: Vec<OrderSnapshot<ExchangeKey, AssetKey, InstrumentKey>>,
}

impl<ExchangeKey, AssetKey, InstrumentKey> AccountSnapshot<ExchangeKey, AssetKey, InstrumentKey> {
    pub fn time_most_recent(&self) -> Option<DateTime<Utc>> {
        let order_times = self.instruments.iter().flat_map(|instrument| {
            instrument
                .orders
                .iter()
                .filter_map(|order| order.state.time_exchange())
        });
        let balance_times = self.balances.iter().map(|balance| balance.time_exchange);

        order_times.chain(balance_times).max()
    }

    pub fn assets(&self) -> impl Iterator<Item = &AssetKey> {
        self.balances.iter().map(|balance| &balance.asset)
    }

    pub fn instruments(&self) -> impl Iterator<Item = &InstrumentKey> {
        self.instruments.iter().map(|snapshot| &snapshot.instrument)
    }
}
