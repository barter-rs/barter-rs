use crate::v2::engine::audit::{AuditKind, Auditor, GeneratedRequestsAudit};
use crate::v2::engine::command::Command;
use crate::v2::engine::error::ExecutionRxDropped;
use crate::v2::order::{Order, OrderId, RequestOpen};
use crate::v2::{
    channel::Tx,
    engine::{
        audit::Audit,
        state::{instrument::OrderManager},
    },
    execution::ExecutionRequest,
    risk::{RiskApproved, RiskManager},
    strategy::Strategy,
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use itertools::{Itertools};
use std::fmt::Debug;
use crate::v2::engine::state::{EngineState, InstrumentStateManager};

pub mod audit;
pub mod command;
pub mod error;
pub mod ext;
pub mod state;
pub mod clock;

// Todo: Must Have:
//  - Utility to re-create state from Audit snapshot + updates w/ interactive mode (backward would require Vec<State> to be created on .next()) (add compression using file system)
//  - All state update implementations:
//  - Add tests for all Managers:
//  - Add interface for user strategy & risk to access Instrument contract
//  - Utility for AssetKey, InstrumentKey lookups, as well as constructing Instruments contracts, etc
//  - Engine functionality can be injected, on_shutdown, on_state_update_error, on_disconnect, etc.
//  - Find suitable place & usage of trait EngineClock.
//   '--> Just Auditor? in EngineState, or perhaps just Engine?

// Todo: Nice To Have:
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

pub trait Processor<Event> {
    type Output;
    fn process(&mut self, event: Event) -> Self::Output;
}

#[derive(Debug, Constructor)]
pub struct Engine<ExecutionTx, InstrumentState, StrategyT, Risk, AssetKey, InstrumentKey>
where
    StrategyT: Strategy<InstrumentState, AssetKey, InstrumentKey>,
    Risk: RiskManager<InstrumentState, AssetKey, InstrumentKey>,
{
    pub sequence: u64,
    pub time: fn() -> DateTime<Utc>,
    pub execution_tx: ExecutionTx,
    pub state: EngineState<InstrumentState, StrategyT::State, Risk::State, AssetKey, InstrumentKey>,
    pub strategy: StrategyT,
    pub risk: Risk,
}

impl<ExecutionTx, InstrumentState, StrategyT, Risk, AssetKey, InstrumentKey, StrategyState, RiskState>
    Processor<&Command<InstrumentKey>>
    for Engine<ExecutionTx, InstrumentState, StrategyT, Risk, AssetKey, InstrumentKey>
where
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = ExecutionRxDropped>,
    InstrumentState: InstrumentStateManager<AssetKey, InstrumentKey>,
    StrategyT: Strategy<InstrumentState, AssetKey, InstrumentKey, State = StrategyState, RiskState = RiskState>,
    StrategyState: Clone,
    Risk: RiskManager<InstrumentState, AssetKey, InstrumentKey, State = RiskState, StrategyState = StrategyState>,
    RiskState: Clone,
    InstrumentKey: Clone
{
    type Output = Result<(), ExecutionRxDropped>;

    fn process(&mut self, event: &Command<InstrumentKey>) -> Self::Output {
        match event {
            Command::Execute(request) => {
                self.execute(request)?;
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

impl<ExecutionTx, InstrumentState, StrategyT, Risk, AssetKey, InstrumentKey, StrategyState, RiskState>
    Engine<ExecutionTx, InstrumentState, StrategyT, Risk, AssetKey, InstrumentKey>
where
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = ExecutionRxDropped>,
    InstrumentState: InstrumentStateManager<AssetKey, InstrumentKey>,
    StrategyT: Strategy<InstrumentState, AssetKey, InstrumentKey, State = StrategyState, RiskState = RiskState>,
    StrategyState: Clone,
    Risk: RiskManager<InstrumentState, AssetKey, InstrumentKey, State = RiskState, StrategyState = StrategyState>,
    RiskState: Clone,
    InstrumentKey: Clone,
{
    pub fn trade(&mut self) -> Result<GeneratedRequestsAudit<InstrumentKey>, ExecutionRxDropped> {
        // Generate orders
        let (cancels, opens) = self.strategy.generate_orders(&self.state);

        // RiskApprove & RiskRefuse order requests
        let (cancels, opens, refused_cancels, refused_opens) =
            self.risk.check(&self.state, cancels, opens);

        // Send risk approved order request
        let (cancels_sent, cancel_send_errs): (Vec<_>, Vec<_>) = cancels
            .into_iter()
            .map(|RiskApproved(cancel)| {
                self.execution_tx
                    .send(ExecutionRequest::Cancel(cancel.clone()))
                    .map(|_| cancel)
            })
            .partition_result();
        let (opens_sent, open_send_errs): (Vec<_>, Vec<_>) = opens
            .into_iter()
            .map(|RiskApproved(open)| {
                self.execution_tx
                    .send(ExecutionRequest::Open(open.clone()))
                    .map(|_| open)
            })
            .partition_result();

        // Collect remaining Iterators
        let refused_cancels = refused_cancels.into_iter().collect::<Vec<_>>();
        let refused_opens = refused_opens.into_iter().collect::<Vec<_>>();

        // Record in flight order requests
        let order_manager = self.state.instrument.orders_mut();
        for request in cancels_sent.iter() {
            order_manager.record_in_flight_cancel(request);
        }
        for request in opens_sent.iter() {
            order_manager.record_in_flight_open(request);
        }

        if !cancel_send_errs.is_empty() | !open_send_errs.is_empty() {
            return Err(ExecutionRxDropped);
        }

        Ok(GeneratedRequestsAudit {
            cancels: cancels_sent,
            opens: opens_sent,
            refused_cancels,
            refused_opens,
        })
    }

    pub fn execute(&mut self, request: &ExecutionRequest<InstrumentKey>) -> Result<(), ExecutionRxDropped> {
        self.execution_tx.send(request.clone())?;

        let order_manager = self.state.instrument.orders_mut();
        match request {
            ExecutionRequest::Cancel(cancel) => {
                order_manager.record_in_flight_cancel(cancel);
            }
            ExecutionRequest::Open(open) => {
                order_manager.record_in_flight_open(open);
            }
        }

        Ok(())
    }

    pub fn close_position(
        &mut self,
        instrument: &InstrumentKey,
    ) -> Result<Vec<Order<InstrumentKey, RequestOpen>>, ExecutionRxDropped> {
        let (opens_sent, open_send_errs): (Vec<_>, Vec<_>) = self
            .strategy
            .close_position_request(instrument, &self.state)
            .into_iter()
            .map(|open| {
                self.execution_tx
                    .send(ExecutionRequest::Open(open.clone()))
                    .map(|_| open)
            })
            .partition_result();

        // Record in flight order requests
        let order_manager = self.state.instrument.orders_mut();
        for request in opens_sent.iter() {
            order_manager.record_in_flight_open(request);
        }

        if !open_send_errs.is_empty() {
            return Err(ExecutionRxDropped);
        }
        todo!()
    }

    pub fn close_all_positions(
        &mut self,
    ) -> Result<Vec<Order<InstrumentKey, RequestOpen>>, ExecutionRxDropped> {
        let (opens_sent, open_send_errs): (Vec<_>, Vec<_>) = self
            .strategy
            .close_all_positions_request(&self.state)
            .into_iter()
            .map(|open| {
                self.execution_tx
                    .send(ExecutionRequest::Open(open.clone()))
                    .map(|_| open)
            })
            .partition_result();

        // Record in flight order requests
        let order_manager = self.state.instrument.orders_mut();
        for request in opens_sent.iter() {
            order_manager.record_in_flight_open(request);
        }

        if !open_send_errs.is_empty() {
            return Err(ExecutionRxDropped);
        }
        todo!()
    }

    pub fn cancel_order_by_id(
        &mut self,
        _instrument: InstrumentKey,
        _id: OrderId,
    ) -> Result<(), ExecutionRxDropped> {
        todo!()
        // self.execution_tx.send(ExecutionRequest::CancelOrder(RequestCancel::new(instrument, id)))
    }

    pub fn cancel_all_orders(&mut self) -> Result<(), ExecutionRxDropped> {
        todo!()
    }
}

impl<ExecutionTx, InstrumentState, StrategyT, Risk, AssetKey, InstrumentKey>
    Engine<ExecutionTx, InstrumentState, StrategyT, Risk, AssetKey, InstrumentKey>
where
    InstrumentState: Clone,
    StrategyT: Strategy<InstrumentState, AssetKey, InstrumentKey>,
    StrategyT::State: Clone,
    Risk: RiskManager<InstrumentState, AssetKey, InstrumentKey>,
    Risk::State: Clone,
    AssetKey: Clone,
    InstrumentKey: Clone,
{
    pub fn send_snapshot_audit<AuditTx, Event, Error>(&mut self, tx: &mut Auditor<AuditTx>)
    where
        AuditTx: Tx<Item = Audit<AuditKind<EngineState<InstrumentState, StrategyT::State, Risk::State, AssetKey, InstrumentKey>, Event, InstrumentKey, Error>>>,
    {
        let audit = self.build_audit(AuditKind::Snapshot(self.state.clone()));
        tx.send(audit);
    }

    pub fn send_audit<AuditTx, Kind>(&mut self, tx: &mut Auditor<AuditTx>, kind: Kind)
    where
        AuditTx: Tx<Item = Audit<Kind>>,
    {
        let audit = self.build_audit(kind);
        tx.send(audit);
    }

    pub fn build_audit<AuditKind>(&mut self, kind: AuditKind) -> Audit<AuditKind> {
        Audit {
            id: self.sequence_fetch_add(),
            time: (self.time)(),
            kind,
        }
    }

    pub fn sequence_fetch_add(&mut self) -> u64 {
        let sequence = self.sequence;
        self.sequence += 1;
        sequence
    }
}
