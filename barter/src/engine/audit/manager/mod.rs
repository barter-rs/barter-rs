use crate::{
    engine::{
        audit::{manager::history::TradingHistory, Audit, AuditTick},
        state::{
            asset::manager::AssetStateManager,
            connectivity::{manager::ConnectivityManager, ConnectivityState, Health},
            StateManager,
        },
        EngineOutput,
    },
    execution::AccountStreamEvent,
    statistic::summary::{
        asset::TearSheetAssetGenerator, InstrumentTearSheetManager, TradingSummaryGenerator,
    },
    EngineEvent,
};
use barter_data::streams::consumer::MarketStreamEvent;
use barter_execution::AccountEventKind;
use barter_instrument::exchange::ExchangeId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::info;

pub mod history;

// Todo: If I had proper deltas I could use much less memory, even computing lazely the prev or
//      next, and only ever holding the "next event" and "current delta"
//      '--> call .collect() and users can aggregate all into a full history of snapshots
//      - Would it make sense to add input Events too?

// Todo:
//  - Consider using an AuditIndex to speed up history State lookups from other events
//      eg/ Position
//   '--> Makes a good case for adding more "audits" for other types of event
//    --> Can attach a "snapshot index" to all events that comes through

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AuditManager<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey> {
    pub snapshot: AuditTick<State>,
    pub summary: TradingSummaryGenerator,
    pub history: TradingHistory<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey>,
}

impl<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey>
    AuditManager<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey>
{
    pub fn new(snapshot: AuditTick<State>, summary: TradingSummaryGenerator) -> Self
    where
        State: Clone,
    {
        Self {
            snapshot: snapshot.clone(),
            summary,
            history: TradingHistory::from(snapshot),
        }
    }

    pub fn run<Audits, AssetKey>(&mut self, feed: &mut Audits) -> Result<(), String>
    where
        State: Clone + StateManager<ExchangeKey, AssetKey, InstrumentKey>,
        State::MarketEventKind: Debug,
        ExchangeKey: Debug,
        InstrumentKey: Debug,
        Audits: Iterator<
            Item = AuditTick<
                Audit<
                    State,
                    EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>,
                    EngineOutput<ExchangeKey, InstrumentKey, OnDisable, OnDisconnect>,
                >,
            >,
        >,
        AssetKey: Debug,
        TradingSummaryGenerator: InstrumentTearSheetManager<InstrumentKey>
            + AssetStateManager<AssetKey, State = TearSheetAssetGenerator>,
    {
        for audit in feed {
            if self.snapshot.sequence >= audit.sequence {
                continue;
            } else {
                self.validate_and_increment_sequence(audit.sequence)?;
                self.snapshot.time_engine = audit.time_engine;
            }

            let shutdown = match audit.data {
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

            if let Some(audit) = shutdown {
                info!(?audit, "Shutdown | AuditManager");
                break;
            }
        }

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

    pub fn update_from_event<AssetKey>(
        &mut self,
        event: EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>,
    ) where
        State: StateManager<ExchangeKey, AssetKey, InstrumentKey>,
        TradingSummaryGenerator: InstrumentTearSheetManager<InstrumentKey>
            + AssetStateManager<AssetKey, State = TearSheetAssetGenerator>,
    {
        match event {
            EngineEvent::Shutdown | EngineEvent::Command(_) => {
                // No action required
            }
            EngineEvent::TradingStateUpdate(trading_state) => {
                self.replica_engine_state_mut()
                    .update_trading_state(trading_state);
            }
            EngineEvent::Account(event) => match event {
                AccountStreamEvent::Reconnecting(exchange) => {
                    self.replica_engine_state_mut()
                        .connectivity_mut(&exchange)
                        .account = Health::Reconnecting;
                }
                AccountStreamEvent::Item(event) => {
                    if self.at_least_one_component_reconnecting() {
                        self.update_from_account_reconnection(&event.exchange);
                    }
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
                        .connectivity_mut(&exchange)
                        .market_data = Health::Reconnecting;
                }
                MarketStreamEvent::Item(event) => {
                    if self.at_least_one_component_reconnecting() {
                        self.update_from_market_reconnection(event.exchange);
                    }
                    self.replica_engine_state_mut().update_from_market(&event);
                }
            },
        }
    }

    pub fn replica_engine_state(&self) -> &State {
        &self.snapshot.data
    }

    fn replica_engine_state_mut(&mut self) -> &mut State {
        &mut self.snapshot.data
    }

    pub fn at_least_one_component_reconnecting(&self) -> bool
    where
        State: ConnectivityManager<ExchangeKey>,
    {
        self.replica_engine_state().global_health() == Health::Reconnecting
    }

    fn update_from_account_reconnection(&mut self, exchange: &ExchangeKey)
    where
        State: ConnectivityManager<ExchangeKey>,
    {
        self.replica_engine_state_mut()
            .connectivity_mut(exchange)
            .account = Health::Healthy;

        if self.all_components_now_healthy() {
            *self.replica_engine_state_mut().global_health_mut() = Health::Healthy;
        }
    }

    pub fn all_components_now_healthy<Key>(&self) -> bool
    where
        State: ConnectivityManager<Key>,
    {
        self.replica_engine_state()
            .connectivities()
            .all(ConnectivityState::all_healthy)
    }

    fn update_from_market_reconnection(&mut self, exchange: ExchangeId)
    where
        State: ConnectivityManager<ExchangeId>,
    {
        self.replica_engine_state_mut()
            .connectivity_mut(&exchange)
            .market_data = Health::Healthy;

        if self.all_components_now_healthy() {
            *self.replica_engine_state_mut().global_health_mut() = Health::Healthy;
        }
    }

    fn tick<T>(&self, kind: T) -> AuditTick<T> {
        AuditTick {
            sequence: self.snapshot.sequence,
            time_engine: self.snapshot.time_engine,
            data: kind,
        }
    }

    pub fn update_from_engine_output(
        &mut self,
        output: EngineOutput<ExchangeKey, InstrumentKey, OnDisable, OnDisconnect>,
    ) {
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
