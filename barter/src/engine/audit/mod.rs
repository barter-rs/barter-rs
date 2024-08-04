use crate::{
    engine::{
        audit::context::EngineContext,
        error::UnrecoverableEngineError,
        state::{instrument::market_data::MarketDataState, EngineState},
        EngineOutput,
    },
    EngineEvent,
};
use barter_integration::collection::one_or_many::OneOrMany;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use shutdown::ShutdownAudit;

pub mod context;
pub mod manager;
pub mod request;
pub mod shutdown;

pub type DefaultAuditTick<
    MarketState: MarketDataState,
    StrategyState,
    RiskState,
    OnTradingDisabled,
    OnDisconnect,
> = AuditTick<
    DefaultAudit<MarketState, StrategyState, RiskState, OnTradingDisabled, OnDisconnect>,
    EngineContext,
>;

pub type DefaultAudit<
    MarketState: MarketDataState,
    StrategyState,
    RiskState,
    OnTradingDisabled,
    OnDisconnect,
> = Audit<
    EngineState<MarketState, StrategyState, RiskState>,
    EngineEvent<MarketState::EventKind>,
    EngineOutput<OnTradingDisabled, OnDisconnect>,
>;

pub type CustomAudit<
    Event,
    MarketState,
    StrategyState,
    RiskState,
    OnTradingDisabled,
    OnDisconnect,
> = Audit<
    EngineState<MarketState, StrategyState, RiskState>,
    Event,
    EngineOutput<OnTradingDisabled, OnDisconnect>,
>;

pub trait Auditor<AuditKind>
where
    AuditKind: From<Self::Snapshot>,
{
    type Context;
    type Snapshot;
    type Shutdown<Event>;

    fn snapshot(&self) -> Self::Snapshot;

    fn audit<Kind>(&mut self, kind: Kind) -> AuditTick<AuditKind, Self::Context>
    where
        AuditKind: From<Kind>;
}

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Default,
    Deserialize,
    Serialize,
    Constructor,
)]
pub struct AuditTick<Kind, Context> {
    pub event: Kind,
    pub context: Context,
}

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Default,
    Deserialize,
    Serialize,
    Constructor,
)]
pub struct AuditIndex(pub usize);

impl AuditIndex {
    pub fn index(&self) -> usize {
        self.0
    }

    pub fn fetch_add(&mut self) -> AuditIndex {
        let index = *self;
        self.0 += 1;
        index
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Audit<State, Event, Output> {
    Snapshot(State),
    Process(Event),
    ProcessWithOutput(Event, Output),
    Shutdown(ShutdownAudit<Event>),
    ShutdownWithOutput(ShutdownAudit<Event>, Output),
}

impl<State, Event, Output> From<&Audit<State, Event, Output>> for Option<ShutdownAudit<Event>>
where
    Event: Clone,
{
    fn from(value: &Audit<State, Event, Output>) -> Self {
        match value {
            Audit::Shutdown(shutdown) => Some(shutdown.clone()),
            Audit::ShutdownWithOutput(shutdown, _) => Some(shutdown.clone()),
            _ => None,
        }
    }
}

impl<State, Event, Output> Audit<State, Event, Output> {
    pub fn snapshot<S>(state: S) -> Self
    where
        S: Into<State>,
    {
        Self::Snapshot(state.into())
    }

    pub fn process<E>(event: E) -> Self
    where
        E: Into<Event>,
    {
        Self::Process(event.into())
    }

    pub fn process_with_trading_disabled<E, Disabled>(event: E, disabled: Disabled) -> Self
    where
        E: Into<Event>,
        Disabled: Into<Output>,
    {
        Self::ProcessWithOutput(event.into(), disabled.into())
    }

    pub fn process_with_output<E, O>(event: E, output: O) -> Self
    where
        E: Into<Event>,
        O: Into<Output>,
    {
        Self::ProcessWithOutput(event.into(), output.into())
    }

    pub fn shutdown_commanded<E>(event: E) -> Self
    where
        E: Into<Event>,
    {
        Self::Shutdown(ShutdownAudit::Commanded(event.into()))
    }

    pub fn shutdown_on_err_with_output<E, O>(
        event: E,
        unrecoverable: OneOrMany<UnrecoverableEngineError>,
        output: O,
    ) -> Self
    where
        E: Into<Event>,
        O: Into<Output>,
    {
        Self::ShutdownWithOutput(
            ShutdownAudit::Error(event.into(), unrecoverable),
            output.into(),
        )
    }
}

impl<Market, Strategy, Risk, Event, Output> From<EngineState<Market, Strategy, Risk>>
    for Audit<EngineState<Market, Strategy, Risk>, Event, Output>
{
    fn from(value: EngineState<Market, Strategy, Risk>) -> Self {
        Self::Snapshot(value)
    }
}
