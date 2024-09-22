use crate::v2::channel::Tx;
use crate::v2::engine::audit::{AuditEvent, AuditEventKind, Auditor, ProcessAudit};
use crate::v2::engine::error::{ExecutionRxDropped};
use crate::v2::engine::{Engine, Processor};
use crate::v2::execution::ExecutionRequest;
use crate::v2::strategy::Strategy;
use crate::v2::{
    engine::command::Command,
    execution::{AccountEvent, AccountEventKind},
};
use barter_data::event::MarketEvent;
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use crate::v2::engine::state::EngineState;
use crate::v2::risk::RiskManager;

pub mod balance;
pub mod channel;
pub mod engine;
pub mod execution;
pub mod instrument;
pub mod market_data;
pub mod order;
pub mod position;
pub mod risk;
pub mod strategy;
pub mod trade;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From)]
pub enum EngineEvent<AssetKey, InstrumentKey> {
    Terminate,
    Command(Command<InstrumentKey>),
    Account(AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>),
    Market(MarketEvent<InstrumentKey>),
}

#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Deserialize,
    Serialize,
    Constructor,
    From,
)]
pub struct Snapshot<T>(pub T);

impl<T> Snapshot<T> {
    pub const fn as_ref(&self) -> Snapshot<&T> {
        let Snapshot(x) = self;
        Snapshot(x)
    }
}

pub fn run<EventFeed, AuditTx, ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>(
    feed: &mut EventFeed,
    audit_tx: &mut Auditor<AuditTx>,
    engine: &mut Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>,
) where
    EventFeed: Iterator<Item = EngineEvent<AssetKey, InstrumentKey>>,
    AuditTx: Tx<Item = AuditEvent<AuditEventKind<State, EngineEvent<AssetKey, InstrumentKey>, InstrumentKey, ExecutionRxDropped>>>,
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = ExecutionRxDropped>,
    State: EngineState<AssetKey, InstrumentKey, StrategyT::State, Risk::State>,
    StrategyT: Strategy<State, InstrumentKey>,
    Risk: RiskManager<State, InstrumentKey>,
    InstrumentKey: Clone,
    Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>:
        for<'a> Processor<&'a Command<InstrumentKey>, Output = Result<ProcessAudit, ExecutionRxDropped>>,
    for<'a> State: Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>, Output = ProcessAudit>
        + Processor<&'a MarketEvent<InstrumentKey>, Output = ProcessAudit>
        + Clone,
{
    // Send initial EngineState snapshot
    engine.send_snapshot_audit(audit_tx);

    // Todo: maybe re-factor this loop to return an AuditEvent in case of "break"

    for event in feed {
        let process_audit = match &event {
            EngineEvent::Terminate => {
                engine.send_termination_audit(audit_tx, event);
                break;
            }
            EngineEvent::Command(command) => {
                let Ok(audit) = engine.process(command) else {
                    engine.send_termination_with_err_audit(audit_tx, event, ExecutionRxDropped);
                    break;
                };
                audit
            }
            EngineEvent::Account(account) => {
                engine.state.process(account)
            }
            EngineEvent::Market(market) => {
                engine.state.process(market)
            }
        };

        if engine.state.trading_enabled() {
            let Ok(requests_audit) = engine.trade() else {
                engine.send_termination_with_err_audit(audit_tx, event, ExecutionRxDropped);
                break;
            };

            engine.send_process_with_trading_audit(audit_tx, event, process_audit, requests_audit);
        } else {
            engine.send_process_audit(audit_tx, event, process_audit)
        }
    }




    // Todo: shutdown operations, ideally by user input w/ on_error, etc. (use "RunBuilder"?)
}

impl<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey> Processor<&Command<InstrumentKey>>
for Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
where
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = ExecutionRxDropped>,
    State: EngineState<AssetKey, InstrumentKey, StrategyT::State, Risk::State>,
    StrategyT: Strategy<State, InstrumentKey>,
    Risk: RiskManager<State, InstrumentKey>,
    InstrumentKey: Clone,
{
    type Output = Result<ProcessAudit, ExecutionRxDropped>;

    fn process(&mut self, event: &Command<InstrumentKey>) -> Self::Output {
        match event {
            Command::EnableTrading => {
                todo!()
            }
            Command::DisableTrading => {
                todo!()
            }
            Command::Execute(request) => {
                // Todo: ack requests, etc.
                //   Maybe custom error _struct_ for ExecutionTx<Error>? can react accordingly
                //    '--> make sure I still send an Audit to AuditSnapshot can still update state
                self.execution_tx.send(request.clone())?;
            }
            Command::ClosePosition(instrument) => {
                let _result = self.close_position(instrument);
            }
            Command::CloseAllPositions => {
                let _result = self.close_all_positions();
            }
            Command::CancelOrderById((instrument, id)) => {
                let _result = self.cancel_order_by_id(instrument.clone(), id.clone());
            }
            Command::CancelAllOrders => {
                let _result = self.cancel_all_orders();
            }
        }

        todo!()
    }
}

// impl<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey, Error> Processor<AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
// for Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
// where
//     State: for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
// {
//     // type Audit = AuditEvent<AuditEventKind<State, AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>, InstrumentKey, Error>>;
//     type Output = ();
//
//     fn process(&mut self, event: AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>) -> Self::Output {
//         let output = self.state.process(&event);
//         // Todo: this may be able to be removed, since so far we are only updating state...
//         todo!()
//     }
// }
//
// impl<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey, Error> Processor<MarketEvent<InstrumentKey>>
// for Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
// where
//     State: for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
// {
//     // type Audit = AuditEvent<AuditEventKind<State, MarketEvent<InstrumentKey>, InstrumentKey, Error>>;
//     type Output = ();
//
//     fn process(&mut self, event: MarketEvent<InstrumentKey>) -> Self::Output {
//         let output = self.state.process(&event);
//         todo!()
//     }
// }
