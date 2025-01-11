use crate::{error::ConnectivityError, order::id::OrderId};
use chrono::{DateTime, Utc};
use derive_more::{Constructor, From};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub enum OrderState {
    Active(ActiveOrderState),
    Inactive(InactiveOrderState),
}

impl OrderState {
    pub fn open_in_flight(state: OpenInFlight) -> Self {
        Self::Active(ActiveOrderState::OpenInFlight(state))
    }

    pub fn open(state: Open) -> Self {
        Self::Active(ActiveOrderState::Open(state))
    }

    pub fn cancel_in_flight(state: CancelInFlight) -> Self {
        Self::Active(ActiveOrderState::CancelInFlight(state))
    }

    pub fn cancelled(state: Cancelled) -> Self {
        Self::Inactive(InactiveOrderState::Cancelled(state))
    }

    pub fn fully_filled() -> Self {
        Self::Inactive(InactiveOrderState::FullyFilled)
    }

    pub fn failed(state: Failed) -> Self {
        Self::Inactive(InactiveOrderState::Failed(state))
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
pub enum InactiveOrderState {
    Cancelled(Cancelled),
    FullyFilled,
    Failed(Failed),
    Expired,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Cancelled {
    pub id: OrderId,
    pub time_exchange: DateTime<Utc>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub enum Failed {
    Rejected(Option<String>),
    Connectivity(ConnectivityError),
}
