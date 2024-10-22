use barter_integration::Side;
use chrono::{DateTime, Utc};
use derive_more::{Constructor, Display, From};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    Display,
    From,
    Constructor,
)]
pub struct ClientOrderId<T = Uuid>(pub T);

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Deserialize,
    Serialize,
    Display,
    From,
    Constructor,
)]
pub struct OrderId<T = String>(pub T);

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Order<InstrumentKey, State> {
    pub instrument: InstrumentKey,
    pub cid: ClientOrderId,
    pub side: Side,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, From)]
pub enum InternalOrderState {
    OpenInFlight(OpenInFlight),
    Open(Open),
    CancelInFlight(CancelInFlight),
}

impl InternalOrderState {
    pub fn order_id(&self) -> Option<OrderId> {
        match self {
            InternalOrderState::OpenInFlight(_) => None,
            InternalOrderState::Open(state) => Some(state.id.clone()),
            InternalOrderState::CancelInFlight(state) => Some(state.id.clone()),
        }
    }

    pub fn is_open_or_in_flight(&self) -> bool {
        matches!(
            self,
            InternalOrderState::OpenInFlight(_) | InternalOrderState::Open(_)
        )
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, From)]
pub enum ExchangeOrderState {
    Open(Open),
    OpenRejected(OpenRejectedReason),
    CancelRejected(CancelRejectedReason),
    Cancelled(Cancelled),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct RequestOpen {
    pub kind: OrderKind,
    pub time_in_force: TimeInForce,
    pub price: f64,
    pub quantity: f64,
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub enum OrderKind {
    Limit,
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub enum TimeInForce {
    GoodUntilCancelled { post_only: bool },
    GoodUntilEndOfDay,
    FillOrKill,
    ImmediateOrCancel,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct RequestCancel {
    pub id: OrderId,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct OpenInFlight;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct Open {
    pub id: OrderId,
    pub time_update: DateTime<Utc>,
    pub price: f64,
    pub quantity: f64,
    pub filled_quantity: f64,
}

impl Open {
    pub fn quantity_remaining(&self) -> f64 {
        self.quantity - self.filled_quantity
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub struct OpenRejectedReason(pub String);

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct CancelInFlight {
    pub id: OrderId,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Cancelled {
    pub id: OrderId,
    pub time_exchange: DateTime<Utc>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub struct CancelRejectedReason(pub String);

impl<InstrumentKey> From<&Order<InstrumentKey, RequestOpen>>
    for Order<InstrumentKey, InternalOrderState>
where
    InstrumentKey: Clone,
{
    fn from(value: &Order<InstrumentKey, RequestOpen>) -> Self {
        let Order {
            instrument,
            cid,
            side,
            state: _,
        } = value;

        Self {
            instrument: instrument.clone(),
            cid: *cid,
            side: *side,
            state: InternalOrderState::OpenInFlight(OpenInFlight),
        }
    }
}

impl<InstrumentKey> From<&Order<InstrumentKey, RequestCancel>>
    for Order<InstrumentKey, InternalOrderState>
where
    InstrumentKey: Clone,
{
    fn from(value: &Order<InstrumentKey, RequestCancel>) -> Self {
        let Order {
            instrument,
            cid,
            side,
            state,
        } = value;

        Self {
            instrument: instrument.clone(),
            cid: *cid,
            side: *side,
            state: InternalOrderState::CancelInFlight(CancelInFlight {
                id: state.id.clone(),
            }),
        }
    }
}

impl<InstrumentKey> From<Order<InstrumentKey, Open>> for Order<InstrumentKey, InternalOrderState> {
    fn from(value: Order<InstrumentKey, Open>) -> Self {
        let Order {
            instrument,
            cid,
            side,
            state,
        } = value;

        Self {
            instrument,
            cid,
            side,
            state: InternalOrderState::Open(state),
        }
    }
}
