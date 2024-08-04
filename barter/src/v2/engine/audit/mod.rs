use crate::v2::{
    engine::state::{instrument::market_data::MarketDataState, EngineState, IndexedEngineState},
    IndexedEngineEvent,
};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};
use chrono::{DateTime, Utc};
use request::ExecutionRequestAudit;
use serde::{Deserialize, Serialize};
use shutdown::ShutdownAudit;

pub mod request;
pub mod shutdown;

pub type DefaultAudit<MarketState: MarketDataState<InstrumentIndex>, StrategyState, RiskState> =
    Audit<
        IndexedEngineState<MarketState, StrategyState, RiskState>,
        IndexedEngineEvent<MarketState::EventKind>,
        ExecutionRequestAudit<ExchangeIndex, InstrumentIndex>,
    >;

pub type CustomAudit<Event, MarketState, StrategyState, RiskState> = Audit<
    IndexedEngineState<MarketState, StrategyState, RiskState>,
    Event,
    ExecutionRequestAudit<ExchangeIndex, InstrumentIndex>,
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
