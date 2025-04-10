use crate::{error::OrderError, order::id::OrderId};
use barter_instrument::{
    asset::{AssetIndex, name::AssetNameExchange},
    instrument::{InstrumentIndex, name::InstrumentNameExchange},
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

    pub fn time_exchange(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Active(active) => match active {
                ActiveOrderState::OpenInFlight(_) => None,
                ActiveOrderState::Open(state) => Some(state.time_exchange),
                ActiveOrderState::CancelInFlight(state) => {
                    state.order.as_ref().map(|order| order.time_exchange)
                }
            },
            Self::Inactive(inactive) => match inactive {
                InactiveOrderState::Cancelled(state) => Some(state.time_exchange),
                _ => None,
            },
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub enum ActiveOrderState {
    OpenInFlight(OpenInFlight),
    Open(Open),
    CancelInFlight(CancelInFlight),
}

impl ActiveOrderState {
    pub fn open_meta(&self) -> Option<&Open> {
        match self {
            Self::OpenInFlight(_) => None,
            Self::Open(open) => Some(open),
            Self::CancelInFlight(cancel) => cancel.order.as_ref(),
        }
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
    pub filled_quantity: Decimal,
}

impl Open {
    pub fn quantity_remaining(&self, initial_quantity: Decimal) -> Decimal {
        initial_quantity - self.filled_quantity
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize, Constructor,
)]
pub struct CancelInFlight {
    pub order: Option<Open>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub enum InactiveOrderState<AssetKey, InstrumentKey> {
    Cancelled(Cancelled),
    FullyFilled,
    OpenFailed(OrderError<AssetKey, InstrumentKey>),
    Expired,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Cancelled {
    pub id: OrderId,
    pub time_exchange: DateTime<Utc>,
}
