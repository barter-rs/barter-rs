use crate::{
    engine::{
        audit::{manager::history::AuditHistory, Audit, AuditTick},
        state::{
            connectivity::Connection, order::in_flight_recorder::InFlightRequestRecorder,
            StateManager,
        },
        EngineOutput,
    },
    execution::AccountStreamEvent,
    EngineEvent,
};
use barter_data::streams::consumer::MarketStreamEvent;
use barter_execution::AccountEventKind;
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt};
use std::fmt::Debug;
use tracing::{error, info};

pub mod history;

// Todo: If I had proper deltas I could use much less memory, even computing lazely the prev or
//      next, and old ever holding the "next event" and "current delta"
//      '--> call .collect() and users can aggregate all into a full history of snapshots
//      - Would it make sense to add input Events too?

// Todo: Add "outputs" to History
//  - Consider using an AuditIndex to speed up history State lookups from other events
//      eg/ Position
//   '--> Makes a good case for adding more "audits" for other types of event
//    --> Can attach a "snapshot index" to all events that comes through
//  - Could change AuditHistory to "TradingSession"

// Todo: Probably move this to a top level crate module, rather than in Engine

// Todo: AuditTick is very similar to AuditEvent
// #[derive(Debug, Clone)]
// pub struct AuditTick<State> {
//     pub sequence: u64,
//     pub state: State,
// }

#[derive(Debug, Clone)]
pub struct AuditManager<State, OnDisable, OnDisconnect, ExchangeKey, AssetKey, InstrumentKey> {
    pub current: AuditTick<State>,
    pub history: AuditHistory<State, OnDisable, OnDisconnect, ExchangeKey, AssetKey, InstrumentKey>,
}

impl<State, OnDisable, OnDisconnect, ExchangeKey, AssetKey, InstrumentKey>
    AuditManager<State, OnDisable, OnDisconnect, ExchangeKey, AssetKey, InstrumentKey>
where
    State: Clone
        + StateManager<ExchangeKey, AssetKey, InstrumentKey>
        + InFlightRequestRecorder<ExchangeKey, InstrumentKey>,
    State::MarketEventKind: Debug,
    ExchangeKey: Debug,
    AssetKey: Debug,
    InstrumentKey: Debug,
{
    pub fn new(snapshot: AuditTick<State>) -> Self
    where
        State: Clone,
    {
        Self {
            current: snapshot.clone(),
            history: AuditHistory {
                states: vec![snapshot],
                commands: vec![],
                trading_disables: vec![],
                disconnections: vec![],
                orders: vec![],
                trades: vec![],
                positions: vec![],
            },
        }
    }

    // So far, this could be sync
    pub async fn run<AuditStream>(&mut self, stream: &mut AuditStream) -> Result<(), String>
    where
        AuditStream: Stream<
                Item = AuditTick<
                    Audit<
                        State,
                        EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>,
                        EngineOutput<ExchangeKey, InstrumentKey, OnDisable, OnDisconnect>,
                    >,
                >,
            > + Unpin,
    {
        while let Some(audit) = stream.next().await {
            if self.current.sequence >= audit.sequence {
                continue;
            } else {
                self.validate_sequence(audit.sequence)?;
                self.current.time_engine = audit.time_engine;
            }

            let shutdown = match audit.kind {
                Audit::Snapshot(snapshot) => {
                    self.current.kind = snapshot;
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

            self.history.states.push(self.current.clone());

            if let Some(audit) = shutdown {
                info!(?audit, "Shutdown | AuditManager");
                break;
            }
        }

        Ok(())
    }

    fn validate_sequence(&mut self, next: u64) -> Result<(), String> {
        if self.current.sequence != next - 1 {
            return Err(format!(
                "AuditManager | out-of-order AuditStream | next: {} does not follow from {}",
                next, self.current.sequence,
            ));
        }

        self.current.sequence = next;
        Ok(())
    }

    fn tick<T>(&self, kind: T) -> AuditTick<T> {
        AuditTick {
            sequence: self.current.sequence,
            time_engine: self.current.time_engine,
            kind,
        }
    }

    pub fn update_from_event(
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
                self.current.kind.update_trading_state(trading_state);
            }
            EngineEvent::Account(event) => match event {
                AccountStreamEvent::Reconnecting(exchange) => {
                    self.current.kind.connectivity_mut(&exchange).account =
                        Connection::Reconnecting;
                }
                AccountStreamEvent::Item(event) => {
                    if let Some(position) = self.current.kind.update_from_account(&event) {
                        self.history.positions.push(self.tick(position));
                    }
                    if let AccountEventKind::Trade(trade) = event.kind {
                        self.history.trades.push(self.tick(trade));
                    }
                }
            },
            EngineEvent::Market(event) => match event {
                MarketStreamEvent::Reconnecting(exchange) => {
                    self.current.kind.connectivity_mut(&exchange).market_data =
                        Connection::Reconnecting;
                }
                MarketStreamEvent::Item(event) => {
                    self.current.kind.update_from_market(&event);
                }
            },
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
                self.history.orders.push(self.tick(output));
            }
        }
    }
}
