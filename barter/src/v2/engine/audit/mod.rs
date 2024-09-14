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
pub struct AuditEvent<State, Event, InstrumentKey, Error> {
    pub id: u64,
    pub time: DateTime<Utc>,
    pub kind: AuditEventKind<State, Event, InstrumentKey, Error>,
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

pub fn build_audit<State, Event, InstrumentKey, Error>(
    sequence: u64,
    time: DateTime<Utc>,
    kind: AuditEventKind<State, Event, InstrumentKey, Error>,
) -> AuditEvent<State, Event, InstrumentKey, Error>
{
    AuditEvent {
        id: sequence,
        time,
        kind,
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Auditor<AuditTx> {
    pub state: ChannelState<AuditTx>,
}

impl<AuditTx, Event, InstrumentKey, State, Error> Auditor<AuditTx>
where
    AuditTx: Tx<Item = AuditEvent<State, Event, InstrumentKey, Error>, Error = Error>,
    Event: Debug,
    InstrumentKey: Debug,
    State: Debug,
{
    pub fn new(audit_tx: AuditTx) -> Self {
        Self {
            state: ChannelState::Active(audit_tx),
        }
    }

    // pub fn build_event(
    //     &mut self,
    //     kind: AuditEventKind<State, Event, InstrumentKey, Error>
    // ) -> AuditEvent<State, Event, InstrumentKey, Error> {
    //     AuditEvent {
    //         id: self.fetch_add(),
    //         time: (self.time)(),
    //         kind,
    //     }
    // }
    //
    // pub fn fetch_add(&mut self) -> u64 {
    //     let id = self.id;
    //     self.id += 1;
    //     id
    // }
    //
    // pub fn build_snapshot(&mut self, state: State) -> AuditEvent<State, Event, InstrumentKey, Error> {
    //     self.build_event(AuditEventKind::Snapshot(state))
    // }
    //
    // pub fn build_update(&mut self, input: Event) -> AuditEvent<State, Event, InstrumentKey, Error> {
    //     self.build_event(AuditEventKind::Update { input })
    // }
    //
    // pub fn build_update_with_requests(
    //     &mut self,
    //     input: Event,
    //     requests: AuditEventKindRequests<InstrumentKey>,
    // ) -> AuditEvent<State, Event, InstrumentKey, Error>
    // {
    //     self.build_event(AuditEventKind::UpdateWithRequests {
    //         input,
    //         requests,
    //     })
    //
    // }
    // pub fn build_error(&mut self, input: Event, error: Error) -> AuditEvent<State, Event, InstrumentKey, Error> {
    //     self.build_event(AuditEventKind::Error {
    //         input,
    //         error
    //     })
    // }

    pub fn send(&mut self, audit: AuditEvent<State, Event, InstrumentKey, Error>) {
        let ChannelState::Active(tx) = &self.state else {
            return;
        };

        if tx.send(audit).is_err() {
            warn!("AuditEvent receiver dropped - Engine audits will no longer be sent");
            self.state = ChannelState::Disabled
        }
    }

    // pub fn audit_snapshot(&mut self, state: State) {
    //     let ChannelState::Active(tx) = &self.state else {
    //         return;
    //     };
    //
    //     let snapshot = self.build_snapshot(state);
    //     self.id += 1;
    //     debug!(audit = ?snapshot, "Engine Auditor generated AuditEvent Snapshot");
    //
    //     if tx.send(snapshot).is_err() {
    //         warn!("AuditEvent receiver dropped - Engine audits will no longer be sent");
    //         self.state = ChannelState::Disabled
    //     }
    // }
    //
    // pub fn build_snapshot(&self, state: State) -> AuditEvent<State, Event, InstrumentKey, Error> {
    //     AuditEvent {
    //         id: self.id,
    //         time: self.time(),
    //         kind: AuditEventKind::Snapshot(state),
    //     }
    // }
    //
    //
    // pub fn audit(&mut self, input: Event) {
    //     let ChannelState::Active(tx) = &self.state else {
    //         return;
    //     };
    //
    //     let update = self.build_update(input, vec![], vec![], vec![], vec![]);
    //     self.id += 1;
    //     debug!(audit = ?update, "Engine Auditor generated AuditEvent Update");
    //
    //     if tx.send(update).is_err() {
    //         warn!("AuditEvent receiver dropped - Engine audits will no longer be sent");
    //         self.state = ChannelState::Disabled
    //     }
    // }
    //
    // pub fn audit_with_orders(
    //     &mut self,
    //     input: Event,
    //     cancels: Vec<RiskApproved<Order<InstrumentKey, RequestCancel>>>,
    //     opens: Vec<RiskApproved<Order<InstrumentKey, RequestOpen>>>,
    //     refused_cancels: Vec<RiskRefused<Order<InstrumentKey, RequestCancel>>>,
    //     refused_opens: Vec<RiskRefused<Order<InstrumentKey, RequestOpen>>>,
    // ) {
    //     let ChannelState::Active(tx) = &self.state else {
    //         return;
    //     };
    //
    //     let update = self.build_update(input, cancels, opens, refused_cancels, refused_opens);
    //     self.id += 1;
    //     debug!(audit = ?update, "Engine Auditor generated AuditEvent Update");
    //
    //     if tx.send(update).is_err() {
    //         warn!("AuditEvent receiver dropped - Engine audits will no longer be sent");
    //         self.state = ChannelState::Disabled
    //     }
    // }
    //
    // pub fn build_update(
    //     &self,
    //     input: Event,
    //     cancels: Vec<RiskApproved<Order<InstrumentKey, RequestCancel>>>,
    //     opens: Vec<RiskApproved<Order<InstrumentKey, RequestOpen>>>,
    //     refused_cancels: Vec<RiskRefused<Order<InstrumentKey, RequestCancel>>>,
    //     refused_opens: Vec<RiskRefused<Order<InstrumentKey, RequestOpen>>>,
    // ) -> AuditEvent<State, Event, InstrumentKey> {
    //     AuditEvent {
    //         id: self.id,
    //         time: self.time(),
    //         kind: AuditEventKind::Update {
    //             input,
    //             cancels,
    //             opens,
    //             refused_cancels,
    //             refused_opens,
    //         },
    //     }
    // }
}
