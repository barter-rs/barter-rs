use crate::{error::OrderError, order::id::OrderId};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex},
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
};
use chrono::{DateTime, Utc};
use derive_more::{Constructor, From};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Convenient type alias for an [`OrderState`] keyed with [`AssetNameExchange`]
/// and [`InstrumentNameExchange`].
pub type UnindexedOrderState = OrderState<AssetNameExchange, InstrumentNameExchange>;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub enum OrderState<AssetKey = AssetIndex, InstrumentKey = InstrumentIndex> {
    Active(ActiveOrderState),
    Inactive(InactiveOrderState<AssetKey, InstrumentKey>),
}

impl<AssetKey, InstrumentKey> OrderState<AssetKey, InstrumentKey> {
    pub fn active<S>(state: S) -> Self
    where
        S: Into<ActiveOrderState>,
    {
        OrderState::Active(state.into())
    }

    pub fn inactive<S>(state: S) -> Self
    where
        S: Into<InactiveOrderState<AssetKey, InstrumentKey>>,
    {
        OrderState::Inactive(state.into())
    }

    pub fn fully_filled() -> Self {
        Self::Inactive(InactiveOrderState::FullyFilled)
    }

    pub fn expired() -> Self {
        Self::Inactive(InactiveOrderState::Expired)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub enum ActiveOrderState {
    OpenInFlight(OpenInFlight),
    Open(Open),
    CancelInFlight(CancelInFlight),
}

impl ActiveOrderState {
    pub fn order_id(&self) -> Option<OrderId> {
        match self {
            ActiveOrderState::OpenInFlight(_) => None,
            ActiveOrderState::Open(state) => Some(state.id.clone()),
            ActiveOrderState::CancelInFlight(state) => state.id.clone(),
        }
    }

    pub fn is_open_or_in_flight(&self) -> bool {
        matches!(
            self,
            ActiveOrderState::OpenInFlight(_) | ActiveOrderState::Open(_)
        )
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct OpenInFlight;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Open {
    pub id: OrderId,
    pub time_exchange: DateTime<Utc>,
    pub price: Decimal,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
}

impl Open {
    pub fn quantity_remaining(&self) -> Decimal {
        self.quantity - self.filled_quantity
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct CancelInFlight {
    pub id: Option<OrderId>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub enum InactiveOrderState<AssetKey, InstrumentKey> {
    Cancelled(Cancelled),
    FullyFilled,
    Failed(OrderError<AssetKey, InstrumentKey>),
    Expired,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Cancelled {
    pub id: OrderId,
    pub time_exchange: DateTime<Utc>,
}
