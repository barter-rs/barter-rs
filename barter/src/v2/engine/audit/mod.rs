use crate::v2::{
    channel::{ChannelState, Tx},
    engine::{error::EngineError, state::EngineState},
    order::{Order, RequestCancel, RequestOpen},
    risk::RiskRefused,
    EngineEvent,
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::warn;

// Todo: may want to move this outside of mod engine, perhaps with all the most general types
pub mod manager;

pub type DefaultAudit<
    InstrumentState,
    BalanceState,
    StrategyState,
    RiskState,
    AssetKey,
    InstrumentKey,
    MarketKind,
> = Audit<
    AuditKind<
        EngineState<
            InstrumentState,
            BalanceState,
            StrategyState,
            RiskState,
            AssetKey,
            InstrumentKey,
        >,
        EngineEvent<AssetKey, InstrumentKey, MarketKind>,
        InstrumentKey,
        EngineError,
    >,
>;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Auditor<AuditTx> {
    pub state: ChannelState<AuditTx>,
}

impl<AuditTx, Audit> Auditor<AuditTx>
where
    AuditTx: Tx<Item = Audit>,
{
    pub fn new(audit_tx: AuditTx) -> Self {
        Self {
            state: ChannelState::Active(audit_tx),
        }
    }

    pub fn send(&mut self, audit: Audit) {
        let ChannelState::Active(tx) = &self.state else {
            return;
        };

        if tx.send(audit).is_err() {
            warn!("AuditEvent receiver dropped - Engine audits will no longer be sent");
            self.state = ChannelState::Disabled
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct Audit<Kind> {
    pub id: u64,
    pub time: DateTime<Utc>,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum AuditKind<State, Event, InstrumentKey, Error> {
    Snapshot(State),
    Process(Event),
    ProcessWithGeneratedRequests(Event, GeneratedRequestsAudit<InstrumentKey>),
    Shutdown(ShutdownAudit<Event, Error>),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GeneratedRequestsAudit<InstrumentKey> {
    pub cancels: Vec<Order<InstrumentKey, RequestCancel>>,
    pub opens: Vec<Order<InstrumentKey, RequestOpen>>,
    pub refused_cancels: Vec<RiskRefused<Order<InstrumentKey, RequestCancel>>>,
    pub refused_opens: Vec<RiskRefused<Order<InstrumentKey, RequestOpen>>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ShutdownAudit<Event, Error> {
    FeedEnded,
    ExecutionEnded,
    AfterEvent(Event),
    WithError(Event, Error),
}
