use crate::v2::{
    channel::Tx,
    engine::{
        audit::{AuditKind, Auditor, DefaultAudit, ShutdownAudit},
        command::Command,
        error::ExecutionRxDropped,
        state::{
            balance::BalanceManager,
            instrument::{
                market_data::MarketDataManager, order::OrderManager, position::PositionManager,
            },
            TradingState, UpdateFromSnapshot,
        },
        Engine, Processor,
    },
    execution::{AccountEvent, AccountEventKind, ExecutionRequest, InstrumentAccountSnapshot},
    risk::RiskManager,
    strategy::Strategy,
};
use barter_data::event::MarketEvent;
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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

// Todo: Must Have:
//  - Utility to re-create state from Audit snapshot + updates w/ interactive mode
//    (backward would require Vec<State> to be created on .next()) (add compression using file system)
//  - All state update implementations
//  - Add tests for all Managers
//  - Engine functionality can be injected, on_shutdown, on_state_update_error, on_disconnect, etc.
//  - Find suitable place & usage of trait EngineClock.
//   '--> Just Auditor? in EngineState, or perhaps just Engine?
//  - Change all state update implementations to add new Entries without key clone if they have not been seen

// Todo: Nice To Have:
//  - Utility for AssetKey, InstrumentKey lookups, as well as constructing Instruments contracts, etc
//  - Sequenced log stream that can enrich logs w/ additional context eg/ InstrumentName
//  - Consider removing duplicate logs when calling instrument.state, state_mut, and also Balances!
//  - Extract methods from impl OrderManager for Orders (eg/ update_from_snapshot covers all bases)
//    '--> also ensure duplication is removed from update_from_open & update_from_cancel
//  - Should I collapse nested VecMap in balances and use eg/ VecMap<ExchangeAssetKey, Balance>
//  - Setup some way to get "diffs" for eg/ should Orders.update_from_order_snapshot return a diff?

// Todo: Nice To Have: OrderManager:
//  - OrderManager update_from_open & update_from_cancel may want to return "in flight failed due to X api reason"
//    '--> eg/ find logic associated with "OrderManager received ExecutionError for Order<InFlight>"
//  - Possible we want a 5m window buffer for "strange order updates" to handle out of orders
//    '--> eg/ adding InFlight, receiving Cancelled, the receiving Open -> ghost orders

// Todo: Open Questions:
//  - Process account,market,risk,strategy may want to return errors, especially risk and strategy

// Todo: Way off:
//  - Could use TradingState like concept to switch between Strategies / run loops
//    '--> this continues that idea of scriptable Engine now we everything takes &mut

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From)]
pub enum EngineEvent<AssetKey, InstrumentKey, MarketKind> {
    Shutdown,
    TradingStateUpdate(TradingState),
    Account(AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>),
    Market(MarketEvent<InstrumentKey, MarketKind>),
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

pub fn run<
    EventFeed,
    AuditTx,
    ExecutionTx,
    InstrumentState,
    BalanceState,
    StrategyT,
    Risk,
    AssetKey,
    InstrumentKey,
    StrategyState,
    RiskState,
>(
    feed: &mut EventFeed,
    audit_tx: &mut Auditor<AuditTx>,
    engine: &mut Engine<
        ExecutionTx,
        InstrumentState,
        BalanceState,
        StrategyT,
        Risk,
        AssetKey,
        InstrumentKey,
    >,
) where
    EventFeed:
        Iterator<Item = EngineEvent<AssetKey, InstrumentKey, InstrumentState::MarketEventKind>>,
    AuditTx: Tx<
        Item = DefaultAudit<
            InstrumentState,
            BalanceState,
            StrategyState,
            RiskState,
            AssetKey,
            InstrumentKey,
            InstrumentState::MarketEventKind,
        >,
    >,
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = ExecutionRxDropped>,
    InstrumentState: Clone
        + UpdateFromSnapshot<Vec<InstrumentAccountSnapshot<InstrumentKey>>>
        + MarketDataManager<InstrumentKey>
        + OrderManager<InstrumentKey>
        + PositionManager<AssetKey, InstrumentKey>,
    BalanceState: Clone + BalanceManager<AssetKey>,
    StrategyT: Strategy<
        InstrumentState,
        BalanceState,
        AssetKey,
        InstrumentKey,
        State = StrategyState,
        RiskState = RiskState,
    >,
    StrategyState: Clone
        + for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
        + for<'a> Processor<&'a MarketEvent<InstrumentKey, InstrumentState::MarketEventKind>>,
    Risk: RiskManager<
        InstrumentState,
        BalanceState,
        AssetKey,
        InstrumentKey,
        State = RiskState,
        StrategyState = StrategyState,
    >,
    RiskState: Clone
        + for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
        + for<'a> Processor<&'a MarketEvent<InstrumentKey, InstrumentState::MarketEventKind>>,
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
