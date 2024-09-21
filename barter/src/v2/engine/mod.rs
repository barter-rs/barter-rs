use crate::v2::{channel::Tx, engine::{
    audit::{AuditEvent},
    state::{instrument::OrderManager, EngineState},
}, execution::ExecutionRequest, risk::{RiskApproved, RiskManager}, strategy::Strategy};
use derive_more::Constructor;
use std::fmt::Debug;
use std::marker::PhantomData;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use crate::v2::engine::audit::{AuditEventKind, AuditEventKindRequests};
use crate::v2::order::{Order, RequestCancel};

pub mod audit;
pub mod command;
pub mod error;
pub mod state;
pub mod ext;

pub trait Processor<Event> {
    type Output;
    fn process(&mut self, event: Event) -> Self::Output;
}

// pub trait StateUpdater<Event> {
//     type Output;
//     type Error: Debug;
//     fn try_update(&mut self, event: Event) -> Result<Self::Output, Self::Error>;
// }

#[derive(Debug, Constructor)]
pub struct Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey> {
    pub sequence: u64,
    pub time: fn() -> DateTime<Utc>, // Todo: does this need state? eg/ adapts internal time based on event time diffs
    pub execution_tx: ExecutionTx,
    pub state: State,
    pub strategy: StrategyT,
    pub risk: Risk,
    pub phantom: PhantomData<(AssetKey, InstrumentKey)>
}

// Todo: What do I want?
//  - Users can define their own events and how the Engine processes them
//   '--> if I extract the EventFeed from the Engine, then users have that flexibility

// impl<Error, ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey> Processor<EngineEvent>
// for Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
// where
//     ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = Error>,
//     State: EngineState<EngineEvent, AssetKey, InstrumentKey, StrategyT::State, Risk::State>,
//     StrategyT: Strategy<State, Event = EngineEvent>,
//     Risk: RiskManager<State, Event = EngineEvent>,
//     InstrumentKey: Clone,
// {
//     type Audit = AuditEvent<AuditEventKind<State, EngineEvent, InstrumentKey, Error>>;
//
//     fn process(&mut self, event: EngineEvent) -> Self::Audit {
//         match self.state.try_update(&event) {
//             Ok(actions) => actions,
//             Err(error) => {
//                 return self.audit(AuditEventKind::Error { event: event.clone(), error} );
//             }
//         };
//
//         let audit = if self.state.trading_enabled() {
//             match self.trade() {
//                 Ok(requests) => {
//                     AuditEventKind::UpdateWithRequests { event, requests }
//                 },
//                 Err(error) => {
//                     AuditEventKind::Error { event, error }
//                 }
//             }
//         } else {
//             AuditEventKind::Update { event }
//         };
//
//         self.audit(audit)
//     }
// }

impl<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
where
    State: Clone,
{
    pub fn audit_snapshot<Event, Error>(&mut self) -> AuditEvent<AuditEventKind<State, Event, InstrumentKey, Error>> {
        AuditEvent::new(
            self.sequence_fetch_add(),
            (self.time)(),
            AuditEventKind::Snapshot(self.state.clone())
        )
    }
}

impl<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey, Event, Error>
    Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
where
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = Error>,
    State: EngineState<AssetKey, InstrumentKey, StrategyT::State, Risk::State>,
    StrategyT: Strategy<State, InstrumentKey, Event = Event>,
    Risk: RiskManager<State, InstrumentKey, Event = Event>,
    InstrumentKey: Clone,
{
    pub fn trade(&mut self) -> Result<AuditEventKindRequests<InstrumentKey>, Error> {
        // Generate orders
        let (
            cancels,
            opens
        ) = self.strategy.generate_orders(&self.state);

        // RiskApprove & RiskRefuse order requests
        let (
            cancels,
            opens,
            refused_cancels,
            refused_opens
        ) = self.risk.check(&self.state, cancels, opens);

        // Send risk approved order request
        let (cancel_oks, cancel_errs): (Vec<_>, Vec<_>) = cancels
            .into_iter()
            .map(|RiskApproved(cancel)| self
                .execution_tx
                .send(ExecutionRequest::Cancel(cancel.clone()))
                .map(|_| cancel)
            )
            .partition_result();

        let (open_oks, open_errs): (Vec<_>, Vec<_>) = opens
            .into_iter()
            .map(|RiskApproved(open)|self
                .execution_tx
                .send(ExecutionRequest::Open(open.clone()))
                .map(|_| open)
            )
            .partition_result();

        // Collect remaining Iterators
        let refused_cancels = refused_cancels.into_iter().collect::<Vec<_>>();
        let refused_opens = refused_opens.into_iter().collect::<Vec<_>>();

        // Record in flight order requests
        let order_manager = self.state.orders_mut();
        for request in cancel_oks.iter() {
            order_manager.record_in_flight_cancel(request);
        }
        for request in open_oks.iter() {
            order_manager.record_in_flight_open(request);
        }

        if !cancel_errs.is_empty() | !open_errs.is_empty() {
            // Todo: return Audit that give requests but also gives the fact ExecutionTx is dropped
            todo!()
        }

        Ok(AuditEventKindRequests {
            cancels: cancel_oks,
            opens: open_oks,
            refused_cancels: refused_cancels.into_iter().collect::<Vec<_>>(),
            refused_opens: refused_opens.into_iter().collect::<Vec<_>>()
        })
    }

    pub fn close_position(&mut self, instrument: InstrumentKey) -> Result<Vec<Order<InstrumentKey, RequestCancel>>, Error> {
        let (open_oks, open_errs): (Vec<_>, Vec<_>) = self.strategy
            .close_position_request(&instrument, &self.state)
            .into_iter()
            .map(|open| self
                .execution_tx
                .send(ExecutionRequest::Open(open.clone()))
                .map(|_| open)
            )
            .partition_result();

        // Record in flight order requests
        let order_manager = self.state.orders_mut();
        for request in open_oks.iter() {
            order_manager.record_in_flight_open(request);
        }

        if !open_errs.is_empty() {
            // Todo: return Audit that give requests but also gives the fact ExecutionTx is dropped
            todo!()
        }
        todo!()
    }

    // fn send_cancel_order_request_for_execution<Error>(
    //     order_manager: &mut impl OrderManager<InstrumentKey>,
    //     cancel: &RiskApproved<Order<InstrumentKey, RequestCancel>>
    // ) -> Result<(), Error> {
    //     todo!()
    // }
    //
    // fn send_open_order_request_for_execution<Error>(
    //     order_manager: &mut impl OrderManager<InstrumentKey>,
    //     cancel: &RiskApproved<Order<InstrumentKey, RequestOpen>>
    // ) -> Result<(), Error> {
    //     todo!()
    // }
    //
    //
    // fn send_order_requests_for_execution<Error>(
    //     &mut self,
    //     cancels: [RiskApproved<Order<InstrumentKey, RequestCancel>>],
    //     opens: [RiskApproved<Order<InstrumentKey, RequestOpen>>],
    // ) -> Result<(), Error>
    // {
    //     let order_manager = self.state.orders_mut();
    //     for RiskApproved(cancel) in cancels {
    //         order_manager.record_in_flight_cancel(&cancel);
    //         self.execution_tx.send(cancel);
    //     }
    //
    //
    //     let cancels = cancels
    //         .iter()
    //         .map(RiskApproved::into_item)
    //         .inspect(|request| {
    //             self.state.orders_mut().record_in_flight_cancel(request);
    //         })
    //         .collect::<ExecutionRequest<InstrumentKey>>();
    //
    //     if !cancels.is_empty() {
    //         self.execution_tx.send(cancels)?;
    //     }
    //
    //     let opens = cancels
    //         .iter()
    //         .map(RiskApproved::into_item)
    //         .inspect(|request| {
    //             self.state.orders_mut().record_in_flight_cancel(request);
    //         })
    //         .collect::<ExecutionRequest<InstrumentKey>>();
    //
    //     if !cancels.is_empty() {
    //         self.execution_tx.send(cancels)?;
    //     }
    //
    //
    //     Ok(())
    // }
}

impl<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
{
    pub fn audit<AuditKind>(&mut self, kind: AuditKind) -> AuditEvent<AuditKind> {
        AuditEvent {
            id: self.sequence_fetch_add(),
            time: (self.time)(),
            kind,
        }
    }

    pub fn sequence_fetch_add(&mut self) -> u64 {
        let sequence = self.sequence;
        self.sequence +=1;
        sequence
    }

    // fn send_approved_orders_for_execution<Error>(
    //     &self,
    //     cancels: &[RiskApproved<Order<InstrumentKey, RequestCancel>>],
    //     opens: &[RiskApproved<Order<InstrumentKey, RequestOpen>>],
    // ) -> Result<(), Error>
    // where
    //     ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = Error>,
    //     InstrumentKey: Clone,
    // {
    //     // Record RiskApproved InFlight orders
    //     self.state.orders_mut().record_in_flights(in_flights);
    //
    //     if !cancels.is_empty() {
    //         self.execution_tx.send(ExecutionRequest::from_iter(
    //             opens.iter().cloned().map(RiskApproved::into_item),
    //         ))?;
    //     }
    //
    //     if !opens.is_empty() {
    //         self.execution_tx.send(ExecutionRequest::from_iter(
    //             opens.iter().cloned().map(RiskApproved::into_item),
    //         ))?;
    //     }
    //
    //     Ok(())
    // }

    // Todo: do I want to send requests as single units, to avoid the collect?
    //    -> what are the pros and cons of this...?
    //    - Command::Execute is less convenient... but could be fine if it's just Vec<Order<_, _>>>
    // pub fn send_order_requests_for_execution<Kind, Request, Error>(
    //     &mut self,
    //     cancels: impl IntoIterator<Item = Order<InstrumentKey, RequestCancel>>,
    //     opens: impl IntoIterator<Item = Order<InstrumentKey, RequestOpen>>,
    // ) -> Result<(), Error>
    // where
    //     ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = Error>,
    //     Request: Into<ExecutionRequest<InstrumentKey>>,
    // {
    //
    //
    //
    //
    //
    //
    //     self.execution_tx.send(request.into())
    // }

    pub fn send_execution_request_old<Request, Error>(
        &mut self,
        request: Request
    ) -> Result<(), Error>
    where
        ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = Error>,
        Request: Into<ExecutionRequest<InstrumentKey>>,
    {



        self.execution_tx.send(request.into())
    }
}