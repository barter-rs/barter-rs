use crate::v2::channel::Tx;
use crate::v2::engine::audit::{AuditKind, Auditor, DefaultAudit, ShutdownAudit};
use crate::v2::engine::error::{ExecutionRxDropped};
use crate::v2::engine::state::{TradingState};
use crate::v2::engine::{Engine, Processor};
use crate::v2::execution::ExecutionRequest;
use crate::v2::risk::RiskManager;
use crate::v2::strategy::Strategy;
use crate::v2::{
    engine::command::Command,
    execution::{AccountEvent, AccountEventKind},
};
use barter_data::event::MarketEvent;
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use crate::v2::engine::state::balance::BalanceManager;
use crate::v2::engine::state::instrument::InstrumentStateManager;

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
    Shutdown,
    TradingStateUpdate(TradingState),
    Account(AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>),
    Market(MarketEvent<InstrumentKey>),
    Command(Command<InstrumentKey>),
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

pub fn run<EventFeed, AuditTx, ExecutionTx, InstrumentState, BalanceState, StrategyT, Risk, AssetKey, InstrumentKey, StrategyState, RiskState>(
    feed: &mut EventFeed,
    audit_tx: &mut Auditor<AuditTx>,
    engine: &mut Engine<ExecutionTx, InstrumentState, BalanceState, StrategyT, Risk, AssetKey, InstrumentKey>,
) where
    EventFeed: Iterator<Item = EngineEvent<AssetKey, InstrumentKey>>,
    AuditTx: Tx<Item = DefaultAudit<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey>>,
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = ExecutionRxDropped>,
    InstrumentState: InstrumentStateManager<InstrumentKey>,
    BalanceState: BalanceManager<AssetKey>,
    StrategyT: Strategy<InstrumentState, BalanceState, AssetKey, InstrumentKey, State = StrategyState, RiskState = RiskState>,
    StrategyState: Clone
        + for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
        + for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
    Risk: RiskManager<InstrumentState, BalanceState, AssetKey, InstrumentKey, State = RiskState, StrategyState = StrategyState>,
    RiskState: Clone
        + for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
        + for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
    AssetKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
    Engine<ExecutionTx, InstrumentState, BalanceState, StrategyT, Risk, AssetKey, InstrumentKey>:
        for<'a> Processor<&'a Command<InstrumentKey>, Output = Result<(), ExecutionRxDropped>>,
{
    // Send initial EngineState snapshot
    engine.send_snapshot_audit(audit_tx);

    let termination_audit = loop {
        let Some(event) = feed.next() else {
            break AuditKind::Shutdown(ShutdownAudit::FeedEnded);
        };

        match &event {
            EngineEvent::Shutdown => break AuditKind::Shutdown(ShutdownAudit::AfterEvent(event)),
            EngineEvent::TradingStateUpdate(trading_state) => {
                engine.state.process(*trading_state);
            }
            EngineEvent::Account(account) => {
                engine.state.process(account);
            }
            EngineEvent::Market(market) => {
                engine.state.process(market);
            }
            EngineEvent::Command(command) => {
                if engine.process(command).is_err() {
                    break AuditKind::Shutdown(ShutdownAudit::ExecutionEnded);
                }
            }
        }

        let audit_kind = if let TradingState::Enabled = engine.state.trading {
            let Ok(generated_requests_audit) = engine.trade() else {
                break AuditKind::Shutdown(ShutdownAudit::ExecutionEnded);
            };
            AuditKind::ProcessWithGeneratedRequests(event, generated_requests_audit)
        } else {
            AuditKind::Process(event)
        };

        engine.send_audit(audit_tx, audit_kind);
    };

    // Todo: add results of shutdown tasks into TerminationAudit
    engine.send_audit(audit_tx, termination_audit);

    // Todo: shutdown operations, ideally by user input w/ on_error, etc. (use "RunBuilder"?)
}
