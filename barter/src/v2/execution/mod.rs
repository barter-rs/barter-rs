use crate::v2::{
    balance::AssetBalance,
    execution::error::{ConnectivityError, ExecutionError},
    order::{
        Cancelled, ClientOrderId, ExchangeOrderState, Open, Order, RequestCancel, RequestOpen,
    },
    position::Position,
    trade::Trade,
    Snapshot,
};
use barter_integration::model::Exchange;
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use vecmap::VecMap;

pub mod error;
pub mod link;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, From)]
pub enum ExecutionRequest<InstrumentKey> {
    CancelOrders(Vec<Order<InstrumentKey, RequestCancel>>),
    OpenOrders(Vec<Order<InstrumentKey, RequestOpen>>),
}

impl<InstrumentKey> ExecutionRequest<InstrumentKey> {
    pub fn is_empty(&self) -> bool {
        match self {
            ExecutionRequest::CancelOrders(cancels) => cancels.is_empty(),
            ExecutionRequest::OpenOrders(opens) => opens.is_empty(),
        }
    }
}

impl<InstrumentKey> FromIterator<Order<InstrumentKey, RequestCancel>>
    for ExecutionRequest<InstrumentKey>
{
    fn from_iter<T: IntoIterator<Item = Order<InstrumentKey, RequestCancel>>>(iter: T) -> Self {
        Self::CancelOrders(iter.into_iter().collect())
    }
}

impl<InstrumentKey> FromIterator<Order<InstrumentKey, RequestOpen>>
    for ExecutionRequest<InstrumentKey>
{
    fn from_iter<T: IntoIterator<Item = Order<InstrumentKey, RequestOpen>>>(iter: T) -> Self {
        Self::OpenOrders(iter.into_iter().collect())
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct AccountEvent<Kind> {
    pub exchange: Exchange,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From)]
pub enum AccountEventKind<AssetKey, InstrumentKey> {
    /// Full [`AccountSnapshot`] - replaces all existing state.
    Snapshot(AccountSnapshot<AssetKey, InstrumentKey>),

    /// Single [`AssetBalance`] snapshot - replaces existing balance state.
    BalanceSnapshot(Snapshot<AssetBalance<AssetKey>>),
    /// Single [`Order<InstrumentKey, Open>`] snapshot - replaces existing order state.
    OrderSnapshot(Snapshot<Order<InstrumentKey, ExchangeOrderState>>),
    /// Single [`Position`] snapshot - replaces existing position state.
    PositionSnapshot(Snapshot<Position<InstrumentKey>>),

    /// Response to an [`Order<InstrumentKey, RequestOpen>`].
    OrderOpened(Order<InstrumentKey, Result<Open, ExecutionError>>),
    /// Response to an [`Order<InstrumentKey, RequestCancel>`].
    OrderCancelled(Order<InstrumentKey, Result<Cancelled, ExecutionError>>),

    /// [`Order<InstrumentKey, Open>`] partial or full fill.
    Trade(Trade<InstrumentKey>),

    /// [`ConnectivityError`] (ie/ non-API error such as disconnected websocket).
    ConnectivityError(ConnectivityError),
}

impl<AssetKey: Eq, InstrumentKey: Eq> AccountEvent<AccountEventKind<AssetKey, InstrumentKey>> {
    pub fn snapshot(self) -> Option<AccountSnapshot<AssetKey, InstrumentKey>> {
        if let AccountEventKind::Snapshot(snapshot) = self.kind {
            Some(snapshot)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct AccountSnapshot<AssetKey, InstrumentKey> {
    pub balances: Vec<AssetBalance<AssetKey>>,
    pub instruments: Vec<InstrumentAccountSnapshot<InstrumentKey>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct InstrumentAccountSnapshot<InstrumentKey> {
    pub position: Position<InstrumentKey>,
    pub orders: Vec<Order<InstrumentKey, Open>>,
}
