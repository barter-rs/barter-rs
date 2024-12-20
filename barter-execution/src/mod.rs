use crate::v2::{
    balance::AssetBalance,
    error::ClientError,
    order::{Cancelled, ExchangeOrderState, Open, Order},
    snapshot::{AccountSnapshot, Snapshot},
    trade::Trade,
};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
};
use derive_more::From;
use serde::{Deserialize, Serialize};

pub mod balance;
pub mod error;
pub mod client;
pub mod order;
pub mod snapshot;
pub mod trade;

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
