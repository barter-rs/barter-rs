use crate::order::{
    id::StrategyId,
    request::{OrderRequestCancel, OrderRequestOpen, RequestCancel, RequestOpen},
    state::UnindexedOrderState,
};
use barter_instrument::{
    Side,
    asset::{AssetIndex, name::AssetNameExchange},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{InstrumentIndex, name::InstrumentNameExchange},
};
use derive_more::{Constructor, Display};
use id::ClientOrderId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use state::{ActiveOrderState, Cancelled, InactiveOrderState, Open, OpenInFlight, OrderState};

/// `Order` related identifiers.
pub mod id;

/// `Order` states.
///
/// eg/ `OpenInFlight`, `Open`, `Rejected`, `Expired`, etc.
pub mod state;

/// Order open and cancel request types.
///
/// ie/ `OrderRequestOpen` & `OrderRequestCancel`.
pub mod request;

/// Convenient type alias for an [`Order`] keyed with [`ExchangeId`] and [`InstrumentNameExchange`].
pub type UnindexedOrder = Order<ExchangeId, InstrumentNameExchange, UnindexedOrderState>;

/// Convenient type alias for an [`OrderKey`] keyed with [`ExchangeId`]
/// and [`InstrumentNameExchange`].
pub type UnindexedOrderKey = OrderKey<ExchangeId, InstrumentNameExchange>;

/// Convenient type alias for an [`OrderSnapshot`] keyed with [`ExchangeId`], [`AssetNameExchange`],
/// and [`InstrumentNameExchange`].
pub type UnindexedOrderSnapshot = Order<
    ExchangeId,
    InstrumentNameExchange,
    OrderState<AssetNameExchange, InstrumentNameExchange>,
>;

/// Convenient type alias for an [`Order`] [`OrderState`] snapshot.
pub type OrderSnapshot<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
> = Order<ExchangeKey, InstrumentKey, OrderState<AssetKey, InstrumentKey>>;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]

pub struct OrderEvent<State, ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    pub key: OrderKey<ExchangeKey, InstrumentKey>,
    pub state: State,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct OrderKey<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    pub exchange: ExchangeKey,
    pub instrument: InstrumentKey,
    pub strategy: StrategyId,
    pub cid: ClientOrderId,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Order<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex, State = OrderState> {
    pub key: OrderKey<ExchangeKey, InstrumentKey>,
    pub side: Side,
    pub price: Decimal,
    pub quantity: Decimal,
    pub kind: OrderKind,
    pub time_in_force: TimeInForce,
    pub state: State,
}

impl<ExchangeKey, AssetKey, InstrumentKey>
    Order<ExchangeKey, InstrumentKey, OrderState<AssetKey, InstrumentKey>>
{
    pub fn to_active(&self) -> Option<Order<ExchangeKey, InstrumentKey, ActiveOrderState>>
    where
        ExchangeKey: Clone,
        InstrumentKey: Clone,
    {
        let OrderState::Active(state) = &self.state else {
            return None;
        };

        Some(Order {
            key: self.key.clone(),
            side: self.side,
            price: self.price,
            quantity: self.quantity,
            kind: self.kind,
            time_in_force: self.time_in_force,
            state: state.clone(),
        })
    }

    pub fn to_inactive(
        &self,
    ) -> Option<Order<ExchangeKey, InstrumentKey, InactiveOrderState<AssetKey, InstrumentKey>>>
    where
        ExchangeKey: Clone,
        AssetKey: Clone,
        InstrumentKey: Clone,
    {
        let OrderState::Inactive(state) = &self.state else {
            return None;
        };

        Some(Order {
            key: self.key.clone(),
            side: self.side,
            price: self.price,
            quantity: self.quantity,
            kind: self.kind,
            time_in_force: self.time_in_force,
            state: state.clone(),
        })
    }
}

impl<ExchangeKey, InstrumentKey> Order<ExchangeKey, InstrumentKey, ActiveOrderState>
where
    ExchangeKey: Clone,
    InstrumentKey: Clone,
{
    pub fn to_request_cancel(&self) -> Option<OrderRequestCancel<ExchangeKey, InstrumentKey>> {
        let Order { key, state, .. } = self;

        let request_cancel = match state {
            ActiveOrderState::OpenInFlight(_) => RequestCancel { id: None },
            ActiveOrderState::Open(open) => RequestCancel {
                id: Some(open.id.clone()),
            },
            _ => return None,
        };

        Some(OrderRequestCancel {
            key: key.clone(),
            state: request_cancel,
        })
    }
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub enum OrderKind {
    Market,
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

impl<ExchangeKey, InstrumentKey> From<&OrderRequestOpen<ExchangeKey, InstrumentKey>>
    for Order<ExchangeKey, InstrumentKey, ActiveOrderState>
where
    ExchangeKey: Clone,
    InstrumentKey: Clone,
{
    fn from(value: &OrderRequestOpen<ExchangeKey, InstrumentKey>) -> Self {
        let OrderRequestOpen {
            key,
            state:
                RequestOpen {
                    side,
                    price,
                    quantity,
                    kind,
                    time_in_force,
                },
        } = value;

        Self {
            key: key.clone(),
            side: *side,
            price: *price,
            quantity: *quantity,
            kind: *kind,
            time_in_force: *time_in_force,
            state: ActiveOrderState::OpenInFlight(OpenInFlight),
        }
    }
}

impl<ExchangeKey, InstrumentKey> From<Order<ExchangeKey, InstrumentKey, Open>>
    for Order<ExchangeKey, InstrumentKey, ActiveOrderState>
{
    fn from(value: Order<ExchangeKey, InstrumentKey, Open>) -> Self {
        let Order {
            key,
            side,
            price,
            quantity,
            kind,
            time_in_force,
            state,
        } = value;

        Self {
            key,
            side,
            price,
            quantity,
            kind,
            time_in_force,
            state: ActiveOrderState::Open(state),
        }
    }
}

impl<ExchangeKey, AssetKey, InstrumentKey> From<Order<ExchangeKey, InstrumentKey, Open>>
    for Order<ExchangeKey, InstrumentKey, OrderState<AssetKey, InstrumentKey>>
{
    fn from(value: Order<ExchangeKey, InstrumentKey, Open>) -> Self {
        let Order {
            key,
            side,
            price,
            quantity,
            kind,
            time_in_force,
            state,
        } = value;

        Self {
            key,
            side,
            price,
            quantity,
            kind,
            time_in_force,
            state: OrderState::Active(ActiveOrderState::Open(state)),
        }
    }
}

impl<ExchangeKey, AssetKey, InstrumentKey> From<Order<ExchangeKey, InstrumentKey, Cancelled>>
    for Order<ExchangeKey, InstrumentKey, OrderState<AssetKey, InstrumentKey>>
{
    fn from(value: Order<ExchangeKey, InstrumentKey, Cancelled>) -> Self {
        let Order {
            key,
            side,
            price,
            quantity,
            kind,
            time_in_force,
            state,
        } = value;

        Self {
            key,
            side,
            price,
            quantity,
            kind,
            time_in_force,
            state: OrderState::Inactive(InactiveOrderState::Cancelled(state)),
        }
    }
}
