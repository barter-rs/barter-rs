use crate::v2::{
    channel::{ChannelState, Tx},
    order::{Order, RequestCancel, RequestOpen},
    risk::{RiskApproved, RiskRefused},
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::{warn};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct AuditEvent<Kind> {
    pub id: u64,
    pub time: DateTime<Utc>,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum AuditEventKind<State, Event, InstrumentKey, Error> {
    Snapshot(State),
    Update {
        event: Event
    },
    UpdateWithRequests {
        event: Event,
        requests: AuditEventKindRequests<InstrumentKey>,
    },
    Error {
        event: Event,
        error: Error,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AuditEventKindRequests<InstrumentKey> {
    pub cancels: Vec<RiskApproved<Order<InstrumentKey, RequestCancel>>>,
    pub opens: Vec<RiskApproved<Order<InstrumentKey, RequestOpen>>>,
    pub refused_cancels: Vec<RiskRefused<Order<InstrumentKey, RequestCancel>>>,
    pub refused_opens: Vec<RiskRefused<Order<InstrumentKey, RequestOpen>>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Auditor<AuditTx> {
    pub state: ChannelState<AuditTx>,
}

impl<AuditTx, Audit> Auditor<AuditTx>
where
    AuditTx: Tx<Item = Audit>
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
