use crate::v2::channel::Tx;
use crate::v2::engine::audit::{AuditEvent, AuditEventKind, Auditor};
use crate::v2::engine::error::EngineError;
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

// Todo: Add user functionality such as on_error, etc inside "default" Engine via Builder or Runner
pub fn run<EventFeed, AuditTx, ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>(
    feed: &mut EventFeed,
    auditor: &mut Auditor<AuditTx>,
    engine: &mut Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>,
) where
    EventFeed: Iterator<Item = EngineEvent<AssetKey, InstrumentKey>>,
    AuditTx: Tx<Item = AuditEvent<AuditEventKind<State, EngineEvent<AssetKey, InstrumentKey>, InstrumentKey, EngineError>>>,
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>>,
    State: EngineState<AssetKey, InstrumentKey, StrategyT::State, Risk::State>,
    StrategyT: Strategy<State, InstrumentKey>,
    Risk: RiskManager<State, InstrumentKey>,
    InstrumentKey: Clone,
    Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>:
        for<'a> Processor<&'a Command<InstrumentKey>>,
    for<'a> State: Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
        + Processor<&'a MarketEvent<InstrumentKey>>
        + Clone,
{
    let snapshot = engine.build_audit(AuditEventKind::Snapshot(engine.state.clone()));
    auditor.send(snapshot);

    for event in feed {
        match event {
            EngineEvent::Terminate => {
                break;
            }
            EngineEvent::Command(command) => {
                engine.process(&command); // AuditEventKind::External?
            }
            EngineEvent::Account(account) => {
                engine.state.process(&account); // AuditEventKind::Update?
            }
            EngineEvent::Market(market) => {
                engine.state.process(&market); // AuditEventKind::Update?
            }
        };

        engine.trade();

        // Todo: generate orders...!
    }
    // Todo: shutdown operations, etc.
}

impl<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey, Error> Processor<&Command<InstrumentKey>>
for Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
where
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = Error>,
    State: EngineState<AssetKey, InstrumentKey, StrategyT::State, Risk::State>,
    StrategyT: Strategy<State, InstrumentKey>,
    Risk: RiskManager<State, InstrumentKey>,
    InstrumentKey: Clone,
{
    type Output = AuditEventKind<State, Command<InstrumentKey>, InstrumentKey, Error>;

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
                let _result = self.execution_tx.send(request.clone());
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
