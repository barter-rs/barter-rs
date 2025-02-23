use crate::{
    EngineEvent,
    engine::{
        EngineMeta, Processor,
        audit::{
            AuditTick, DefaultAuditTick, EngineAudit, ProcessAudit, context::EngineContext,
            shutdown::ShutdownAudit,
        },
        state::{EngineState, instrument::market_data::MarketDataState},
    },
    execution::AccountStreamEvent,
};
use barter_data::{event::MarketEvent, streams::consumer::MarketStreamEvent};
use barter_execution::AccountEvent;
use barter_instrument::instrument::InstrumentIndex;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::{info, info_span};

pub const AUDIT_REPLICA_STATE_UPDATE_SPAN_NAME: &str = "audit_replica_state_update_span";

/// Manages a replica of an `EngineState` instance by processing AuditStream events produced by
/// the `Engine`.
///
/// Useful for supporting non-hot path trading system components such as UIs, web apps, etc.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct StateReplicaManager<State> {
    pub meta_start: EngineMeta,
    pub state_replica: AuditTick<State, EngineContext>,
}

impl<MarketState, StrategyState, RiskState>
    StateReplicaManager<EngineState<MarketState, StrategyState, RiskState>>
where
    MarketState: MarketDataState,
    StrategyState: for<'a> Processor<&'a AccountEvent>
        + for<'a> Processor<&'a MarketEvent<InstrumentIndex, MarketState::EventKind>>,
    RiskState: for<'a> Processor<&'a AccountEvent>
        + for<'a> Processor<&'a MarketEvent<InstrumentIndex, MarketState::EventKind>>,
{
    /// Construct a new `StateReplicaManager` using the provided `EngineState` snapshot as a seed.
    pub fn new(
        snapshot: AuditTick<EngineState<MarketState, StrategyState, RiskState>, EngineContext>,
    ) -> Self {
        Self {
            meta_start: EngineMeta {
                time_start: snapshot.context.time,
                sequence: snapshot.context.sequence,
            },
            state_replica: snapshot,
        }
    }

    /// Run the `StateReplicaManager`, managing a replica of an `EngineState` instance by processing
    /// AuditStream events produced by an `Engine`.
    pub fn run<AuditIter, OnDisable, OnDisconnect>(
        &mut self,
        feed: &mut AuditIter,
    ) -> Result<(), String>
    where
        AuditIter: Iterator<
            Item = DefaultAuditTick<MarketState, StrategyState, RiskState, OnDisable, OnDisconnect>,
        >,
        OnDisable: Debug,
        OnDisconnect: Debug,
    {
        info!("StateReplicaManager running");

        // Create Tracing Span used to filter duplicate replica EngineState update logs
        let audit_span = info_span!(AUDIT_REPLICA_STATE_UPDATE_SPAN_NAME);
        let audit_span_guard = audit_span.enter();

        let shutdown_audit = loop {
            let Some(audit) = feed.next() else {
                break ShutdownAudit::FeedEnded;
            };

            if self.state_replica.context.sequence >= audit.context.sequence {
                continue;
            } else {
                self.validate_and_update_context(audit.context)?;
            }

            let shutdown_audit = match audit.event {
                EngineAudit::Snapshot(snapshot) => {
                    self.state_replica.event = snapshot;
                    None
                }
                EngineAudit::Process(ProcessAudit::Process(event)) => {
                    self.update_from_event(event);
                    None
                }
                EngineAudit::Process(ProcessAudit::ProcessWithOutput(event, _)) => {
                    self.update_from_event(event);
                    None
                }
                EngineAudit::Shutdown(shutdown) => Some(shutdown),
            };

            if let Some(shutdown_audit) = shutdown_audit {
                break shutdown_audit;
            }
        };

        // End Tracing Span used to filter duplicate EngineState update logs
        drop(audit_span_guard);

        info!(?shutdown_audit, "AuditManager stopped");

        Ok(())
    }

    fn validate_and_update_context(&mut self, next: EngineContext) -> Result<(), String> {
        if self.state_replica.context.sequence.value() != next.sequence.value() - 1 {
            return Err(format!(
                "AuditManager | out-of-order AuditStream | next: {:?} does not follow from {:?}",
                next.sequence, self.state_replica.context.sequence,
            ));
        }

        self.state_replica.context = next;
        Ok(())
    }

    /// Updates the internal `EngineState` using the provided `EngineEvent`.
    pub fn update_from_event(&mut self, event: EngineEvent<MarketState::EventKind>) {
        match event {
            EngineEvent::Shutdown | EngineEvent::Command(_) => {
                // No action required
            }
            EngineEvent::TradingStateUpdate(trading_state) => {
                let _audit = self
                    .replica_engine_state_mut()
                    .trading
                    .update(trading_state);
            }
            EngineEvent::Account(event) => match event {
                AccountStreamEvent::Reconnecting(exchange) => {
                    self.replica_engine_state_mut()
                        .connectivity
                        .update_from_account_reconnecting(&exchange);
                }
                AccountStreamEvent::Item(event) => {
                    self.replica_engine_state_mut().update_from_account(&event);
                }
            },
            EngineEvent::Market(event) => match event {
                MarketStreamEvent::Reconnecting(exchange) => {
                    self.replica_engine_state_mut()
                        .connectivity
                        .update_from_market_reconnecting(&exchange);
                }
                MarketStreamEvent::Item(event) => {
                    self.replica_engine_state_mut().update_from_market(&event);
                }
            },
        }
    }

    /// Returns a reference to the `EngineState` replica.
    pub fn replica_engine_state(&self) -> &EngineState<MarketState, StrategyState, RiskState> {
        &self.state_replica.event
    }

    fn replica_engine_state_mut(
        &mut self,
    ) -> &mut EngineState<MarketState, StrategyState, RiskState> {
        &mut self.state_replica.event
    }
}
