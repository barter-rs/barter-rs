use crate::{
    engine::{
        audit::{
            context::EngineContext, manager::history::TradingHistory, shutdown::ShutdownAudit,
            Audit, AuditTick, DefaultAuditTick,
        },
        state::{instrument::market_data::MarketDataState, EngineState},
        EngineOutput, Processor,
    },
    execution::AccountStreamEvent,
    statistic::summary::{
        AssetTearSheetManager, InstrumentTearSheetManager, TradingSummaryGenerator,
    },
    EngineEvent,
};
use barter_data::{event::MarketEvent, streams::consumer::MarketStreamEvent};
use barter_execution::{AccountEvent, AccountEventKind};
use barter_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::{info, info_span};

pub mod history;

pub const AUDIT_REPLICA_STATE_UPDATE_SPAN_NAME: &str = "audit_replica_state_update_span";

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AuditManager<State, OnDisable, OnDisconnect> {
    pub snapshot: AuditTick<State, EngineContext>,
    pub summary: TradingSummaryGenerator,
    pub history: TradingHistory<State, OnDisable, OnDisconnect, ExchangeIndex, InstrumentIndex>,
}

impl<MarketState, StrategyState, RiskState, OnDisable, OnDisconnect>
    AuditManager<EngineState<MarketState, StrategyState, RiskState>, OnDisable, OnDisconnect>
where
    MarketState: MarketDataState,
    StrategyState: Clone
        + for<'a> Processor<&'a AccountEvent>
        + for<'a> Processor<&'a MarketEvent<InstrumentIndex, MarketState::EventKind>>,
    RiskState: Clone
        + for<'a> Processor<&'a AccountEvent>
        + for<'a> Processor<&'a MarketEvent<InstrumentIndex, MarketState::EventKind>>,
    TradingSummaryGenerator:
        InstrumentTearSheetManager<InstrumentIndex> + AssetTearSheetManager<AssetIndex>,
{
    pub fn new(
        snapshot: AuditTick<EngineState<MarketState, StrategyState, RiskState>, EngineContext>,
        summary: TradingSummaryGenerator,
    ) -> Self {
        Self {
            snapshot: snapshot.clone(),
            summary,
            history: TradingHistory::from(snapshot),
        }
    }

    pub fn run<Audits>(&mut self, feed: &mut Audits) -> Result<(), String>
    where
        Audits: Iterator<
            Item = DefaultAuditTick<MarketState, StrategyState, RiskState, OnDisable, OnDisconnect>,
        >,
    {
        info!("AuditManager running");

        // Create Tracing Span used to filter duplicate replica EngineState update logs
        let audit_span = info_span!(AUDIT_REPLICA_STATE_UPDATE_SPAN_NAME);
        let audit_span_guard = audit_span.enter();

        let shutdown_audit = loop {
            let Some(audit) = feed.next() else {
                break ShutdownAudit::FeedEnded;
            };

            if self.snapshot.context.sequence >= audit.context.sequence {
                continue;
            } else {
                self.validate_and_update_context(audit.context)?;
            }

            let AuditTick { event, context } = audit;

            let shutdown_audit = match event {
                Audit::Snapshot(snapshot) => {
                    self.snapshot.event = snapshot;
                    None
                }
                Audit::Process(event) => {
                    self.update_from_event(event, context);
                    None
                }
                Audit::ProcessWithOutput(event, output) => {
                    self.update_from_event(event, context);
                    self.update_from_engine_output(output, context);
                    None
                }
                Audit::Shutdown(shutdown) => Some(shutdown),
                Audit::ShutdownWithOutput(shutdown, output) => {
                    self.update_from_engine_output(output, context);
                    Some(shutdown)
                }
            };

            self.history
                .add_state_snapshot(self.snapshot.event.clone(), self.snapshot.context);

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
        if self.snapshot.context.sequence.value() != next.sequence.value() - 1 {
            return Err(format!(
                "AuditManager | out-of-order AuditStream | next: {:?} does not follow from {:?}",
                next.sequence, self.snapshot.context.sequence,
            ));
        }

        self.snapshot.context = next;
        Ok(())
    }

    pub fn update_from_event(
        &mut self,
        event: EngineEvent<MarketState::EventKind>,
        context: EngineContext,
    ) {
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
                    if let Some(position) =
                        self.replica_engine_state_mut().update_from_account(&event)
                    {
                        self.summary.update_from_position(&position);
                        self.history.add_position(position, context);
                    }
                    if let AccountEventKind::BalanceSnapshot(balance) = &event.kind {
                        self.summary.update_from_balance(balance.as_ref());
                    }
                    if let AccountEventKind::Trade(trade) = event.kind {
                        self.history.add_trade(trade, context);
                    }
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

    pub fn replica_engine_state(&self) -> &EngineState<MarketState, StrategyState, RiskState> {
        &self.snapshot.event
    }

    fn replica_engine_state_mut(
        &mut self,
    ) -> &mut EngineState<MarketState, StrategyState, RiskState> {
        &mut self.snapshot.event
    }

    pub fn update_from_engine_output(
        &mut self,
        output: EngineOutput<OnDisable, OnDisconnect>,
        context: EngineContext,
    ) {
        match output {
            EngineOutput::Commanded(output) => {
                self.history.add_command_output(output, context);
            }
            EngineOutput::OnTradingDisabled(output) => {
                self.history.add_trading_disabled_output(output, context);
            }
            EngineOutput::OnDisconnect(output) => {
                self.history.add_disconnection_output(output, context);
            }
            EngineOutput::AlgoOrders(output) => {
                self.history.add_orders_sent(output, context);
            }
        }
    }
}
