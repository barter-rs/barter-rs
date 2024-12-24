use crate::{
    engine::{
        audit::{
            manager::history::TradingHistory, shutdown::ShutdownAudit, Audit, AuditTick,
            DefaultAudit,
        },
        state::{
            asset::manager::AssetStateManager, instrument::market_data::MarketDataState,
            EngineState,
        },
        EngineOutput, Processor,
    },
    execution::AccountStreamEvent,
    statistic::summary::{
        asset::TearSheetAssetGenerator, InstrumentTearSheetManager, TradingSummaryGenerator,
    },
    EngineEvent,
};
use barter_data::{event::MarketEvent, streams::consumer::MarketStreamEvent};
use barter_execution::{AccountEvent, AccountEventKind};
use barter_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::info;

pub mod history;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AuditManager<State, OnDisable, OnDisconnect> {
    pub snapshot: AuditTick<State>,
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
    TradingSummaryGenerator: InstrumentTearSheetManager<InstrumentIndex>
        + AssetStateManager<AssetIndex, State = TearSheetAssetGenerator>,
{
    pub fn new(
        snapshot: AuditTick<EngineState<MarketState, StrategyState, RiskState>>,
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
            Item = AuditTick<
                DefaultAudit<MarketState, StrategyState, RiskState, OnDisable, OnDisconnect>,
            >,
        >,
    {
        info!("AuditManager running");

        let shutdown_audit = loop {
            let Some(audit) = feed.next() else {
                break ShutdownAudit::FeedEnded;
            };

            if self.snapshot.sequence >= audit.sequence {
                continue;
            } else {
                self.validate_and_increment_sequence(audit.sequence)?;
                self.snapshot.time_engine = audit.time_engine;
            }

            let shutdown_audit = match audit.data {
                Audit::Snapshot(snapshot) => {
                    self.snapshot.data = snapshot;
                    None
                }
                Audit::Process(event) => {
                    self.update_from_event(event);
                    None
                }
                Audit::ProcessWithOutput(event, output) => {
                    self.update_from_event(event);
                    self.update_from_engine_output(output);
                    None
                }
                Audit::Shutdown(shutdown) => Some(shutdown),
                Audit::ShutdownWithOutput(shutdown, output) => {
                    self.update_from_engine_output(output);
                    Some(shutdown)
                }
            };

            self.history.states.push(self.snapshot.clone());

            if let Some(shutdown_audit) = shutdown_audit {
                break shutdown_audit;
            }
        };

        info!(?shutdown_audit, "AuditManager stopped");

        Ok(())
    }

    fn validate_and_increment_sequence(&mut self, next: u64) -> Result<(), String> {
        if self.snapshot.sequence != next - 1 {
            return Err(format!(
                "AuditManager | out-of-order AuditStream | next: {} does not follow from {}",
                next, self.snapshot.sequence,
            ));
        }

        self.snapshot.sequence = next;
        Ok(())
    }

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
                    if let Some(position) =
                        self.replica_engine_state_mut().update_from_account(&event)
                    {
                        self.summary.update_from_position(&position);
                        self.history.positions.push(self.tick(position));
                    }
                    if let AccountEventKind::BalanceSnapshot(balance) = &event.kind {
                        self.summary.update_from_balance(balance.as_ref());
                    }
                    if let AccountEventKind::Trade(trade) = event.kind {
                        self.history.trades.push(self.tick(trade));
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
        &self.snapshot.data
    }

    fn replica_engine_state_mut(
        &mut self,
    ) -> &mut EngineState<MarketState, StrategyState, RiskState> {
        &mut self.snapshot.data
    }

    fn tick<T>(&self, kind: T) -> AuditTick<T> {
        AuditTick {
            sequence: self.snapshot.sequence,
            time_engine: self.snapshot.time_engine,
            data: kind,
        }
    }

    pub fn update_from_engine_output(&mut self, output: EngineOutput<OnDisable, OnDisconnect>) {
        match output {
            EngineOutput::Commanded(output) => {
                self.history.commands.push(self.tick(output));
            }
            EngineOutput::OnTradingDisabled(output) => {
                self.history.trading_disables.push(self.tick(output));
            }
            EngineOutput::OnDisconnect(output) => {
                self.history.disconnections.push(self.tick(output));
            }
            EngineOutput::AlgoOrders(output) => {
                self.history.orders_sent.push(self.tick(output));
            }
        }
    }
}
