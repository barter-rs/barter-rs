use crate::v2::{
    balance::AssetBalance,
    execution::error::ClientError,
    order::{Cancelled, ExchangeOrderState, Open, Order, RequestCancel, RequestOpen},
    position::PositionExchange,
    trade::Trade,
    Snapshot,
};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};

pub mod builder;
pub mod error;
pub mod manager;
pub mod map;

/// Convenient type alias for an [`AccountEvent`] keyed with [`ExchangeIndex`], [`AssetIndex`]
/// and [`InstrumentIndex`].
pub type IndexedAccountEvent = AccountEvent<ExchangeIndex, AssetIndex, InstrumentIndex>;

/// Convenient type alias for an [`AccountEvent`] keyed with [`ExchangeId`],
/// [`AssetNameExchange`], and [`InstrumentNameExchange`].
pub type UnindexedAccountEvent =
    AccountEvent<ExchangeId, AssetNameExchange, InstrumentNameExchange>;

/// Convenient type alias for an [`AccountSnapshot`] keyed with [`ExchangeIndex`], [`AssetIndex`]
/// and [`InstrumentIndex`].
pub type IndexedAccountSnapshot = AccountSnapshot<ExchangeIndex, AssetIndex, InstrumentIndex>;

/// Convenient type alias for an [`AccountSnapshot`] keyed with [`ExchangeId`],
/// [`AssetNameExchange`], and [`InstrumentNameExchange`].
pub type UnindexedAccountSnapshot =
    AccountSnapshot<ExchangeId, AssetNameExchange, InstrumentNameExchange>;

/// Convenient type alias for an [`ExecutionRequest`] keyed with [`ExchangeIndex`], [`AssetIndex`]
/// and [`InstrumentIndex`].
pub type IndexedExecutionRequest = ExecutionRequest<ExchangeIndex, InstrumentIndex>;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, From)]
pub enum ExecutionRequest<ExchangeKey, InstrumentKey> {
    Cancel(Order<ExchangeKey, InstrumentKey, RequestCancel>),
    Open(Order<ExchangeKey, InstrumentKey, RequestOpen>),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AccountEvent<ExchangeKey, AssetKey, InstrumentKey> {
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

    /// Single [`PositionExchange`] snapshot - used to update internal position state.
    PositionSnapshot(Snapshot<PositionExchange<InstrumentKey>>),

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
    Trade(Trade<AssetKey, InstrumentKey>),
}

impl<ExchangeKey, AssetKey: Eq, InstrumentKey: Eq>
    AccountEvent<ExchangeKey, AssetKey, InstrumentKey>
{
    pub fn snapshot(self) -> Option<AccountSnapshot<ExchangeKey, AssetKey, InstrumentKey>> {
        if let AccountEventKind::Snapshot(snapshot) = self.kind {
            Some(snapshot)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct AccountSnapshot<ExchangeKey, AssetKey, InstrumentKey> {
    pub balances: Vec<AssetBalance<AssetKey>>,
    pub instruments: Vec<InstrumentAccountSnapshot<ExchangeKey, InstrumentKey>>,
}

// Todo: Maybe InstrumentAccountState, AssetAccountState, and use same types as Engine

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentAccountSnapshot<ExchangeKey, InstrumentKey> {
    pub position: PositionExchange<InstrumentKey>,
    pub orders: Vec<Order<ExchangeKey, InstrumentKey, ExchangeOrderState>>,
}

impl<ExchangeKey, AssetKey, InstrumentKey> AccountSnapshot<ExchangeKey, AssetKey, InstrumentKey> {
    pub fn assets(&self) -> impl Iterator<Item = &AssetKey> {
        self.balances.iter().map(|balance| &balance.asset)
    }

    pub fn instruments(&self) -> impl Iterator<Item = &InstrumentKey> {
        self.instruments
            .iter()
            .map(|snapshot| &snapshot.position.instrument)
    }
}
