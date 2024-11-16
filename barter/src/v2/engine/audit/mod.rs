use crate::v2::{
    engine::{
        error::UnrecoverableEngineError,
        state::{instrument::market_data::MarketDataState, EngineState, IndexedEngineState},
        IndexedEngineOutput,
    },
    IndexedEngineEvent,
};
use barter_instrument::instrument::InstrumentIndex;
use barter_integration::collection::one_or_many::OneOrMany;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shutdown::ShutdownAudit;

pub mod request;
pub mod shutdown;

pub type DefaultAudit<
    MarketState: MarketDataState<InstrumentIndex>,
    StrategyState,
    RiskState,
    OnTradingDisabled,
    OnDisconnect,
> = Audit<
    IndexedEngineState<MarketState, StrategyState, RiskState>,
    IndexedEngineEvent<MarketState::EventKind>,
    IndexedEngineOutput<OnTradingDisabled, OnDisconnect>,
>;

pub type CustomAudit<
    Event,
    MarketState,
    StrategyState,
    RiskState,
    OnTradingDisabled,
    OnDisconnect,
> = Audit<
    IndexedEngineState<MarketState, StrategyState, RiskState>,
    Event,
    IndexedEngineOutput<OnTradingDisabled, OnDisconnect>,
>;

pub trait Auditor<AuditKind>
where
    AuditKind: From<Self::Snapshot>,
{
    type Snapshot;
    fn snapshot(&self) -> Self::Snapshot;
    fn build_audit<Kind>(&mut self, kind: Kind) -> AuditEvent<AuditKind>
    where
        AuditKind: From<Kind>;
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AuditEvent<Kind> {
    pub id: u64,
    pub time: DateTime<Utc>,
    pub kind: Kind,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Audit<State, Event, Output> {
    Snapshot(State),
    Process(Event),
    ProcessWithOutput(Event, Output),
    Shutdown(ShutdownAudit<Event>),
    ShutdownWithOutput(ShutdownAudit<Event>, Output),
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

impl<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey, Event, Output>
    From<EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>>
    for Audit<
        EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>,
        Event,
        Output,
    >
{
    fn from(
        value: EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> Self {
        Self::Snapshot(value)
    }
}
