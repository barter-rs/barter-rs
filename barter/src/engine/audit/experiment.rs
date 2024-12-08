use crate::{
    engine::{
        audit::{Audit, AuditEvent},
        state::{
            connectivity::Connection, order::in_flight_recorder::InFlightRequestRecorder,
            position::PositionExited, StateManager,
        },
        EngineOutput,
    },
    execution::AccountStreamEvent,
    EngineEvent,
};
use barter_data::streams::consumer::MarketStreamEvent;
use barter_execution::trade::Trade;
use futures::{Stream, StreamExt};
use std::fmt::Debug;
use tracing::{error, info};
// Todo: Add "outputs" to History
//  - Consider using an AuditIndex to speed up history State lookups from other events
//      eg/ Position
//   '--> Makes a good case for adding more "audits" for other types of event
//    --> Can attach a "snapshot index" to all events that comes through

#[derive(Debug, Clone)]
pub struct AuditSnapshotManager<State, AssetKey, InstrumentKey> {
    pub current: AuditSnapshot<State>,
    pub history: Vec<AuditSnapshot<State>>,
    pub trades: Vec<AuditSnapshot<Trade<AssetKey, InstrumentKey>>>,
    pub positions: Vec<AuditSnapshot<PositionExited<AssetKey, InstrumentKey>>>,
}

#[derive(Debug, Clone)]
pub struct AuditSnapshot<State> {
    pub audit_sequence: u64,
    pub state: State,
}

impl<State, AssetKey, InstrumentKey> AuditSnapshotManager<State, AssetKey, InstrumentKey> {
    pub fn new(snapshot: AuditSnapshot<State>) -> Self
    where
        State: Clone,
    {
        Self {
            current: snapshot.clone(),
            history: vec![snapshot],
            trades: vec![],
            positions: vec![],
        }
    }

    pub async fn run<AuditStream, ExchangeKey, OnTradingDisabled, OnDisconnect>(
        mut self,
        stream: &mut AuditStream,
    ) -> Result<Self, Self>
    where
        State: Clone
            + StateManager<ExchangeKey, AssetKey, InstrumentKey>
            + InFlightRequestRecorder<ExchangeKey, InstrumentKey>,
        State::MarketEventKind: Debug,
        ExchangeKey: Debug,
        AssetKey: Debug,
        InstrumentKey: Debug,
        AuditStream: Stream<
                Item = AuditEvent<
                    Audit<
                        State,
                        EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>,
                        EngineOutput<
                            ExchangeKey,
                            AssetKey,
                            InstrumentKey,
                            OnTradingDisabled,
                            OnDisconnect,
                        >,
                    >,
                >,
            > + Unpin,
    {
        while let Some(audit) = stream.next().await {
            if self.current.audit_sequence >= audit.sequence {
                continue;
            } else if let Err(error) = self.validate_sequence(audit.sequence) {
                error!(%error, "AuditSnapshotManager encountered out-of-order AuditStream");
                return Err(self);
            }

            let shutdown = match audit.kind {
                Audit::Snapshot(snapshot) => {
                    let _ = std::mem::replace(&mut self.current.state, snapshot);
                    None
                }
                Audit::Process(event) => {
                    self.update_from_event(event);
                    None
                }
                Audit::ProcessWithOutput(event, _output) => {
                    self.update_from_event(event);
                    None
                }
                Audit::Shutdown(shutdown) => Some(shutdown),
                Audit::ShutdownWithOutput(shutdown, _output) => Some(shutdown),
            };

            self.history.push(self.current.clone());

            if let Some(audit) = shutdown {
                info!(?audit, "Shutdown | AuditSnapshotManager");
                break;
            }
        }

        Ok(self)
    }

    fn validate_sequence(&mut self, next: u64) -> Result<(), String> {
        if self.current.audit_sequence != next - 1 {
            return Err(format!(
                "next: {} does not follow from {}",
                next, self.current.audit_sequence,
            ));
        }

        self.current.audit_sequence = next;
        Ok(())
    }

    pub fn update_from_event<ExchangeKey>(
        &mut self,
        event: EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>,
    ) where
        State: StateManager<ExchangeKey, AssetKey, InstrumentKey>,
    {
        match event {
            EngineEvent::Shutdown | EngineEvent::Command(_) => {
                // No action required
            }
            EngineEvent::TradingStateUpdate(trading_state) => {
                self.current.state.update_trading_state(trading_state);
            }
            EngineEvent::Account(event) => match event {
                AccountStreamEvent::Reconnecting(exchange) => {
                    self.current.state.connectivity_mut(&exchange).account =
                        Connection::Reconnecting;
                }
                AccountStreamEvent::Item(event) => {
                    if let Some(position) = self.current.state.update_from_account(&event) {
                        self.positions.push(AuditSnapshot {
                            audit_sequence: self.current.audit_sequence,
                            state: position,
                        });
                    }
                }
            },
            EngineEvent::Market(event) => match event {
                MarketStreamEvent::Reconnecting(exchange) => {
                    self.current.state.connectivity_mut(&exchange).market_data =
                        Connection::Reconnecting;
                }
                MarketStreamEvent::Item(event) => {
                    self.current.state.update_from_market(&event);
                }
            },
        }
    }
}
