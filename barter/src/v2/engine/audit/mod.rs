use crate::v2::{
    channel::{ChannelState, Tx},
    engine::error::EngineError,
    order::{Order, RequestCancel, RequestOpen},
    risk::{RiskApproved, RiskRefused},
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::{debug, warn};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct AuditEvent<State, Event, InstrumentKey> {
    pub id: u64,
    pub time: DateTime<Utc>,
    pub kind: AuditEventKind<State, Event, InstrumentKey>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum AuditEventKind<State, Event, InstrumentKey> {
    Snapshot(State),
    Update {
        input: Event,
        cancels: Vec<RiskApproved<Order<InstrumentKey, RequestCancel>>>,
        opens: Vec<RiskApproved<Order<InstrumentKey, RequestOpen>>>,
        refused_cancels: Vec<RiskRefused<Order<InstrumentKey, RequestCancel>>>,
        refused_opens: Vec<RiskRefused<Order<InstrumentKey, RequestOpen>>>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
pub struct Auditor<AuditTx> {
    pub id: u64,
    pub state: ChannelState<AuditTx>,
}

impl<AuditTx, Event, InstrumentKey, State> Auditor<AuditTx>
where
    AuditTx: Tx<Item = AuditEvent<State, Event, InstrumentKey>, Error = EngineError>,
    Event: Debug,
    InstrumentKey: Debug,
    State: Debug,
{
    pub fn new(audit_tx: AuditTx) -> Self {
        Self {
            id: 1,
            state: ChannelState::Active(audit_tx),
        }
    }

    pub fn audit_snapshot(&mut self, state: State) {
        let ChannelState::Active(tx) = &self.state else {
            return;
        };

        let snapshot = self.build_snapshot(state);
        self.id += 1;
        debug!(audit = ?snapshot, "Engine Auditor generated AuditEvent Snapshot");

        if tx.send(snapshot).is_err() {
            warn!("AuditEvent receiver dropped - Engine audits will no longer be sent");
            self.state = ChannelState::Disabled
        }
    }

    pub fn build_snapshot(&self, state: State) -> AuditEvent<State, Event, InstrumentKey> {
        AuditEvent {
            id: self.id,
            time: Utc::now(),
            kind: AuditEventKind::Snapshot(state),
        }
    }

    pub fn audit(
        &mut self,
        input: Event,
        cancels: Vec<RiskApproved<Order<InstrumentKey, RequestCancel>>>,
        opens: Vec<RiskApproved<Order<InstrumentKey, RequestOpen>>>,
        refused_cancels: Vec<RiskRefused<Order<InstrumentKey, RequestCancel>>>,
        refused_opens: Vec<RiskRefused<Order<InstrumentKey, RequestOpen>>>,
    ) {
        let ChannelState::Active(tx) = &self.state else {
            return;
        };

        let update = self.build_update(input, cancels, opens, refused_cancels, refused_opens);
        self.id += 1;
        debug!(audit = ?update, "Engine Auditor generated AuditEvent Update");

        if tx.send(update).is_err() {
            warn!("AuditEvent receiver dropped - Engine audits will no longer be sent");
            self.state = ChannelState::Disabled
        }
    }

    pub fn build_update(
        &self,
        input: Event,
        cancels: Vec<RiskApproved<Order<InstrumentKey, RequestCancel>>>,
        opens: Vec<RiskApproved<Order<InstrumentKey, RequestOpen>>>,
        refused_cancels: Vec<RiskRefused<Order<InstrumentKey, RequestCancel>>>,
        refused_opens: Vec<RiskRefused<Order<InstrumentKey, RequestOpen>>>,
    ) -> AuditEvent<State, Event, InstrumentKey> {
        AuditEvent {
            id: self.id,
            time: Utc::now(),
            kind: AuditEventKind::Update {
                input,
                cancels,
                opens,
                refused_cancels,
                refused_opens,
            },
        }
    }
}
