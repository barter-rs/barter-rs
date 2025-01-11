use crate::order::{
    id::{OrderId, StrategyId},
    state::UnindexedOrderState,
};
use barter_instrument::{
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
    Side,
};
use derive_more::{Constructor, Display, From};
use id::ClientOrderId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use state::{
    ActiveOrderState, CancelInFlight, Cancelled, InactiveOrderState, Open, OpenInFlight, OrderState,
};

/// `Order` related identifiers.
pub mod id;

/// `Order` states.
///
/// eg/ `OpenInFlight`, `Open`, `Rejected`, `Expired`, etc.
pub mod state;

/// Convenient type alias for an [`Order`] keyed with [`ExchangeId`] and [`InstrumentNameExchange`].
pub type UnindexedOrder = Order<ExchangeId, InstrumentNameExchange, UnindexedOrderState>;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Order<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex, State = OrderState> {
    pub exchange: ExchangeKey,
    pub instrument: InstrumentKey,
    pub strategy: StrategyId,
    pub cid: ClientOrderId,
    pub side: Side,
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
            exchange: self.exchange.clone(),
            instrument: self.instrument.clone(),
            strategy: self.strategy.clone(),
            cid: self.cid.clone(),
            side: self.side,
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
            exchange: self.exchange.clone(),
            instrument: self.instrument.clone(),
            strategy: self.strategy.clone(),
            cid: self.cid.clone(),
            side: self.side,
            state: state.clone(),
        })
    }
}

impl<ExchangeKey, InstrumentKey> Order<ExchangeKey, InstrumentKey, ActiveOrderState>
where
    ExchangeKey: Clone,
    InstrumentKey: Clone,
{
    pub fn to_request_cancel(&self) -> Option<Order<ExchangeKey, InstrumentKey, RequestCancel>> {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = self;

        let request_cancel = match state {
            ActiveOrderState::OpenInFlight(_) => RequestCancel { id: None },
            ActiveOrderState::Open(open) => RequestCancel {
                id: Some(open.id.clone()),
            },
            _ => return None,
        };

        Some(Order {
            exchange: exchange.clone(),
            instrument: instrument.clone(),
            strategy: strategy.clone(),
            cid: cid.clone(),
            side: *side,
            state: request_cancel,
        })
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct RequestOpen {
    pub kind: OrderKind,
    pub time_in_force: TimeInForce,
    pub price: Decimal,
    pub quantity: Decimal,
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

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct RequestCancel {
    pub id: Option<OrderId>,
}

impl<ExchangeKey, InstrumentKey> From<&Order<ExchangeKey, InstrumentKey, RequestOpen>>
    for Order<ExchangeKey, InstrumentKey, ActiveOrderState>
where
    ExchangeKey: Clone,
    InstrumentKey: Clone,
{
    fn from(value: &Order<ExchangeKey, InstrumentKey, RequestOpen>) -> Self {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state: _,
        } = value;

        Self {
            exchange: exchange.clone(),
            instrument: instrument.clone(),
            strategy: strategy.clone(),
            cid: cid.clone(),
            side: *side,
            state: ActiveOrderState::OpenInFlight(OpenInFlight),
        }
    }
}

impl<ExchangeKey, InstrumentKey> From<&Order<ExchangeKey, InstrumentKey, RequestCancel>>
    for Order<ExchangeKey, InstrumentKey, ActiveOrderState>
where
    ExchangeKey: Clone,
    InstrumentKey: Clone,
{
    fn from(value: &Order<ExchangeKey, InstrumentKey, RequestCancel>) -> Self {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = value;

        Self {
            exchange: exchange.clone(),
            instrument: instrument.clone(),
            strategy: strategy.clone(),
            cid: cid.clone(),
            side: *side,
            state: ActiveOrderState::CancelInFlight(CancelInFlight {
                id: state.id.clone(),
            }),
        }
    }
}

impl<ExchangeKey, InstrumentKey> From<Order<ExchangeKey, InstrumentKey, Open>>
    for Order<ExchangeKey, InstrumentKey, ActiveOrderState>
{
    fn from(value: Order<ExchangeKey, InstrumentKey, Open>) -> Self {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = value;

        Self {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state: ActiveOrderState::Open(state),
        }
    }
}

impl<ExchangeKey, AssetKey, InstrumentKey> From<Order<ExchangeKey, InstrumentKey, Open>>
    for Order<ExchangeKey, InstrumentKey, OrderState<AssetKey, InstrumentKey>>
{
    fn from(value: Order<ExchangeKey, InstrumentKey, Open>) -> Self {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = value;

        Self {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state: OrderState::Active(ActiveOrderState::Open(state)),
        }
    }
}

impl<ExchangeKey, AssetKey, InstrumentKey> From<Order<ExchangeKey, InstrumentKey, Cancelled>>
    for Order<ExchangeKey, InstrumentKey, OrderState<AssetKey, InstrumentKey>>
{
    fn from(value: Order<ExchangeKey, InstrumentKey, Cancelled>) -> Self {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = value;

        Self {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state: OrderState::Inactive(InactiveOrderState::Cancelled(state)),
        }
    }
}
