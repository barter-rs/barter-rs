use crate::{error::OrderError, order::id::OrderId};
use barter_instrument::{
    asset::{AssetIndex, name::AssetNameExchange},
    instrument::{InstrumentIndex, name::InstrumentNameExchange},
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Convenient type alias for an [`OrderState`] keyed with [`AssetNameExchange`]
/// and [`InstrumentNameExchange`].
pub type UnindexedOrderState = OrderState<AssetNameExchange, InstrumentNameExchange>;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum OrderState<AssetKey = AssetIndex, InstrumentKey = InstrumentIndex> {
    OpenInFlight(OpenInFlight),
    Open(Open),
    CancelInFlight(CancelInFlight),
    Cancelled(Cancelled),
    FullyFilled(FullyFilled),
    OpenFailed(OrderError<AssetKey, InstrumentKey>),
    Expired(Expired),
}

impl<AssetKey, InstrumentKey> OrderState<AssetKey, InstrumentKey> {
    pub fn order_id(&self) -> Option<&OrderId> {
        match self {
            OrderState::Open(open) => Some(&open.id),
            OrderState::CancelInFlight(cancel) => cancel.order.as_ref().map(|o| &o.id),
            OrderState::Cancelled(cancelled) => Some(&cancelled.id),
            OrderState::OpenInFlight(_)
            | OrderState::FullyFilled(_)
            | OrderState::OpenFailed(_)
            | OrderState::Expired(_) => None,
        }
    }

    pub fn time_exchange(&self) -> Option<DateTime<Utc>> {
        match self {
            OrderState::Open(open) => Some(open.time_exchange),
            OrderState::Cancelled(cancelled) => Some(cancelled.time_exchange),
            _ => None,
        }
    }

    pub fn open_meta(&self) -> Option<&Open> {
        match self {
            OrderState::Open(open) => Some(open),
            OrderState::CancelInFlight(cancel) => cancel.order.as_ref(),
            _ => None,
        }
    }

    pub fn is_active(&self) -> bool {
        match self {
            OrderState::OpenInFlight(_) | OrderState::Open(_) | OrderState::CancelInFlight(_) => {
                true
            }
            OrderState::Cancelled(_)
            | OrderState::FullyFilled(_)
            | OrderState::OpenFailed(_)
            | OrderState::Expired(_) => false,
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

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Cancelled {
    pub id: OrderId,
    pub time_exchange: DateTime<Utc>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct FullyFilled;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Expired;
