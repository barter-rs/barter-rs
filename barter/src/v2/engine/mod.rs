use crate::v2::{
    channel::Tx,
    engine::{
        audit::{Audit, AuditKind, Auditor, DefaultAudit, GeneratedRequestsAudit},
        command::Command,
        error::ExecutionRxDropped,
        state::{
            instrument::{market_data::MarketDataManager, order::OrderManager},
            EngineState,
        },
    },
    execution::ExecutionRequest,
    order::{Order, OrderId, RequestOpen},
    risk::{RiskApproved, RiskManager},
    strategy::Strategy,
};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use itertools::Itertools;
use std::fmt::Debug;

pub mod audit;
pub mod clock;
pub mod command;
pub mod error;
pub mod ext;
pub mod state;

pub trait Processor<Event> {
    type Output;
    fn process(&mut self, event: Event) -> Self::Output;
}

#[derive(Debug, Constructor)]
pub struct Engine<
    ExecutionTx,
    InstrumentState,
    BalanceState,
    StrategyT,
    Risk,
    AssetKey,
    InstrumentKey,
> where
    StrategyT: Strategy<InstrumentState, BalanceState, AssetKey, InstrumentKey>,
    Risk: RiskManager<InstrumentState, BalanceState, AssetKey, InstrumentKey>,
{
    pub sequence: u64,
    pub time: fn() -> DateTime<Utc>,
    pub execution_tx: ExecutionTx,
    pub state: EngineState<
        InstrumentState,
        BalanceState,
        StrategyT::State,
        Risk::State,
        AssetKey,
        InstrumentKey,
    >,
    pub strategy: StrategyT,
    pub risk: Risk,
}

impl<
        ExecutionTx,
        InstrumentState,
        BalanceState,
        StrategyT,
        Risk,
        AssetKey,
        InstrumentKey,
        StrategyState,
        RiskState,
    > Processor<&Command<InstrumentKey>>
    for Engine<ExecutionTx, InstrumentState, BalanceState, StrategyT, Risk, AssetKey, InstrumentKey>
where
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = ExecutionRxDropped>,
    InstrumentState: OrderManager<InstrumentKey>,
    StrategyT: Strategy<
        InstrumentState,
        BalanceState,
        AssetKey,
        InstrumentKey,
        State = StrategyState,
        RiskState = RiskState,
    >,
    StrategyState: Clone,
    Risk: RiskManager<
        InstrumentState,
        BalanceState,
        AssetKey,
        InstrumentKey,
        State = RiskState,
        StrategyState = StrategyState,
    >,
    RiskState: Clone,
    InstrumentKey: Clone,
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

impl<
        ExecutionTx,
        InstrumentState,
        BalanceState,
        StrategyT,
        Risk,
        AssetKey,
        InstrumentKey,
        StrategyState,
        RiskState,
    > Engine<ExecutionTx, InstrumentState, BalanceState, StrategyT, Risk, AssetKey, InstrumentKey>
where
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = ExecutionRxDropped>,
    InstrumentState: OrderManager<InstrumentKey>,
    StrategyT: Strategy<
        InstrumentState,
        BalanceState,
        AssetKey,
        InstrumentKey,
        State = StrategyState,
        RiskState = RiskState,
    >,
    StrategyState: Clone,
    Risk: RiskManager<
        InstrumentState,
        BalanceState,
        AssetKey,
        InstrumentKey,
        State = RiskState,
        StrategyState = StrategyState,
    >,
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
        for request in cancels_sent.iter() {
            self.state.instruments.record_in_flight_cancel(request);
        }
        for request in opens_sent.iter() {
            self.state.instruments.record_in_flight_open(request);
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

    pub fn execute(
        &mut self,
        request: &ExecutionRequest<InstrumentKey>,
    ) -> Result<(), ExecutionRxDropped> {
        self.execution_tx.send(request.clone())?;

        match request {
            ExecutionRequest::Cancel(cancel) => {
                self.state.instruments.record_in_flight_cancel(cancel);
            }
            ExecutionRequest::Open(open) => {
                self.state.instruments.record_in_flight_open(open);
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
        for request in opens_sent.iter() {
            self.state.instruments.record_in_flight_open(request);
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
        for request in opens_sent.iter() {
            self.state.instruments.record_in_flight_open(request);
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

impl<ExecutionTx, InstrumentState, BalanceState, StrategyT, Risk, AssetKey, InstrumentKey>
    Engine<ExecutionTx, InstrumentState, BalanceState, StrategyT, Risk, AssetKey, InstrumentKey>
where
    InstrumentState: Clone + MarketDataManager<InstrumentKey>,
    BalanceState: Clone,
    StrategyT: Strategy<InstrumentState, BalanceState, AssetKey, InstrumentKey>,
    StrategyT::State: Clone,
    Risk: RiskManager<InstrumentState, BalanceState, AssetKey, InstrumentKey>,
    Risk::State: Clone,
    AssetKey: Clone,
    InstrumentKey: Clone,
{
    pub fn send_snapshot_audit<AuditTx>(&mut self, tx: &mut Auditor<AuditTx>)
    where
        AuditTx: Tx<
            Item = DefaultAudit<
                InstrumentState,
                BalanceState,
                StrategyT::State,
                Risk::State,
                AssetKey,
                InstrumentKey,
                InstrumentState::MarketEventKind,
            >,
        >,
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
