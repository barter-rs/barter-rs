use barter_instrument::Side;
use chrono::{DateTime, Utc};
use derive_more::{Constructor, Display, From};
use rand::seq::SliceRandom;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display, From,
)]
pub struct ClientOrderId<T = SmolStr>(pub T);

impl ClientOrderId<SmolStr> {
    /// Construct a `ClientOrderId` from the specified string.
    ///
    /// Use [`Self::random`] to generate a random stack-allocated `ClientOrderId`.
    pub fn new<S: Into<SmolStr>>(id: S) -> Self {
        Self(id.into())
    }

    /// Construct a stack-allocated `ClientOrderId` backed by a 23 byte [`SmolStr`].
    pub fn random() -> Self {
        const LEN_URL_SAFE_SYMBOLS: usize = 64;
        const URL_SAFE_SYMBOLS: [char; LEN_URL_SAFE_SYMBOLS] = [
            '_', '-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e',
            'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v',
            'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
            'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
        ];
        // SmolStr can be up to 23 bytes long without allocating
        const LEN_NON_ALLOCATING_CID: usize = 23;

        let mut thread_rng = rand::thread_rng();

        let random_utf8: [u8; LEN_NON_ALLOCATING_CID] = std::array::from_fn(|_| {
            let symbol = URL_SAFE_SYMBOLS
                .choose(&mut thread_rng)
                .expect("URL_SAFE_SYMBOLS slice is not empty");

            *symbol as u8
        });

        let random_utf8_str =
            std::str::from_utf8(&random_utf8).expect("URL_SAFE_SYMBOLS are valid utf8");

        Self(SmolStr::new_inline(random_utf8_str))
    }
}

impl Default for ClientOrderId<SmolStr> {
    fn default() -> Self {
        Self::random()
    }
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display, From,
)]
pub struct OrderId<T = SmolStr>(pub T);

impl OrderId {
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Self(SmolStr::new(id))
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display, From,
)]
pub struct StrategyId(pub SmolStr);

impl StrategyId {
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Self(SmolStr::new(id))
    }

    pub fn unknown() -> Self {
        Self::new("unknown")
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Order<ExchangeKey, InstrumentKey, State> {
    pub exchange: ExchangeKey,
    pub instrument: InstrumentKey,
    pub strategy: StrategyId,
    pub cid: ClientOrderId,
    pub side: Side,
    pub state: State,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
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
            InternalOrderState::CancelInFlight(state) => state.id.clone(),
        }
    }

    pub fn is_open_or_in_flight(&self) -> bool {
        matches!(
            self,
            InternalOrderState::OpenInFlight(_) | InternalOrderState::Open(_)
        )
    }
}

impl<ExchangeKey, InstrumentKey> Order<ExchangeKey, InstrumentKey, InternalOrderState>
where
    ExchangeKey: Clone,
    InstrumentKey: Clone,
{
    pub fn as_exchange(&self) -> Option<Order<ExchangeKey, InstrumentKey, ExchangeOrderState>> {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state: InternalOrderState::Open(open),
        } = self
        else {
            return None;
        };

        Some(Order {
            exchange: exchange.clone(),
            instrument: instrument.clone(),
            strategy: strategy.clone(),
            cid: cid.clone(),
            side: *side,
            state: ExchangeOrderState::Open(open.clone()),
        })
    }

    pub fn as_request_cancel(&self) -> Option<Order<ExchangeKey, InstrumentKey, RequestCancel>> {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = self;

        let request_cancel = match state {
            InternalOrderState::OpenInFlight(_) => RequestCancel { id: None },
            InternalOrderState::Open(open) => RequestCancel {
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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub enum ExchangeOrderState {
    Open(Open),
    FullyFilled,
    Cancelled(Cancelled),
    Rejected(Option<String>),
    Expired,
}

impl<ExchangeKey, InstrumentKey> Order<ExchangeKey, InstrumentKey, ExchangeOrderState> {
    pub fn as_open(&self) -> Option<Order<ExchangeKey, InstrumentKey, Open>>
    where
        ExchangeKey: Clone,
        InstrumentKey: Clone,
    {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state: ExchangeOrderState::Open(open),
        } = self
        else {
            return None;
        };

        Some(Order {
            exchange: exchange.clone(),
            instrument: instrument.clone(),
            strategy: strategy.clone(),
            cid: cid.clone(),
            side: *side,
            state: open.clone(),
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

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Cancelled {
    pub id: OrderId,
    pub time_exchange: DateTime<Utc>,
}

impl<ExchangeKey, InstrumentKey> From<&Order<ExchangeKey, InstrumentKey, RequestOpen>>
    for Order<ExchangeKey, InstrumentKey, InternalOrderState>
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
            state: InternalOrderState::OpenInFlight(OpenInFlight),
        }
    }
}

impl<ExchangeKey, InstrumentKey> From<&Order<ExchangeKey, InstrumentKey, RequestCancel>>
    for Order<ExchangeKey, InstrumentKey, InternalOrderState>
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
            state: InternalOrderState::CancelInFlight(CancelInFlight {
                id: state.id.clone(),
            }),
        }
    }
}

impl<ExchangeKey, InstrumentKey> From<Order<ExchangeKey, InstrumentKey, Open>>
    for Order<ExchangeKey, InstrumentKey, InternalOrderState>
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
            state: InternalOrderState::Open(state),
        }
    }
}

impl<ExchangeKey, InstrumentKey> From<Order<ExchangeKey, InstrumentKey, Open>>
    for Order<ExchangeKey, InstrumentKey, ExchangeOrderState>
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
            state: ExchangeOrderState::Open(state),
        }
    }
}

impl<ExchangeKey, InstrumentKey> From<Order<ExchangeKey, InstrumentKey, Cancelled>>
    for Order<ExchangeKey, InstrumentKey, ExchangeOrderState>
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
            state: ExchangeOrderState::Cancelled(state),
        }
    }
}
