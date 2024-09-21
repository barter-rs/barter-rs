use crate::v2::{channel::Tx, engine::{
    audit::{AuditEvent, Auditor},
    state::{instrument::OrderManager, EngineState},
}, execution::ExecutionRequest, order::{OpenInFlight, Order, RequestCancel, RequestOpen}, risk::{RiskApproved, RiskManager}, strategy::Strategy, EngineEvent};
use derive_more::Constructor;
use std::fmt::Debug;
use std::marker::PhantomData;
use chrono::{DateTime, Utc};
use crate::v2::engine::audit::{AuditEventKind, AuditEventKindRequests};

pub mod audit;
pub mod command;
pub mod error;
pub mod state;
pub mod ext;

pub trait Processor<Event, Error> {
    type Audit;

    fn audit_snapshot(&mut self) -> Self::Audit;
    fn process(&mut self, event: Event) -> Self::Audit;
}

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

impl<Error, ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey> Processor<EngineEvent, Error>
for Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
where
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = Error>,
    State: EngineState<EngineEvent, AssetKey, InstrumentKey, StrategyT::State, Risk::State, Error = Error>,
    StrategyT: Strategy<State, Event = EngineEvent>,
    Risk: RiskManager<State, Event = EngineEvent>,
    InstrumentKey: Clone,
{
    type Audit = AuditEvent<AuditEventKind<State, EngineEvent, InstrumentKey, Error>>;

    fn audit_snapshot(&mut self) -> Self::Audit {
        AuditEvent::new(
            self.sequence_fetch_add(),
            (self.time)(),
            AuditEventKind::Snapshot(self.state.clone())
        )
    }

    fn process(&mut self, event: EngineEvent) -> Self::Audit {
        match self.state.try_update(&event) {
            Ok(actions) => actions,
            Err(error) => {
                return self.audit(AuditEventKind::Error { event: event.clone(), error} );
            }
        };

        let audit = if self.state.trading_enabled() {
            match self.trade() {
                Ok(requests) => {
                    AuditEventKind::UpdateWithRequests { event, requests }
                },
                Err(error) => {
                    AuditEventKind::Error { event, error }
                }
            }
        } else {
            AuditEventKind::Update { event }
        };

        self.audit(audit)
    }
}

impl<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey, Event, Error>
    Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>
where
    Self: Processor<Event, Error>,
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = Error>,
    State: EngineState<Event, AssetKey, InstrumentKey, StrategyT::State, Risk::State, Error = Error>,
    StrategyT: Strategy<State, Event = Event>,
    Risk: RiskManager<State, Event = Event>,
    InstrumentKey: Clone,
{
    pub fn run<EventFeed, AuditTx>(
        &mut self,
        mut feed: EventFeed,
        mut auditor: Auditor<AuditTx>,
    ) -> Result<(), Error>
    where
        EventFeed: Iterator<Item = Event>,
        AuditTx: Tx<Item = <Self as Processor<Event, Error>>::Audit, Error = Error>,
    {
        let snapshot = self.audit_snapshot();
        auditor.send(snapshot);

        // Todo: Add user functionality such as on_error, etc inside Engine via Builder or Runner

        while let Some(event) = feed.next() {
            let audit = self.process(event);
            auditor.send(audit)
        }

        Ok(())
    }

    fn trade(&mut self) -> Result<AuditEventKindRequests<InstrumentKey>, Error> {
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

        // Generate InFlights for RiskApproved orders
        let (in_flights, opens): (Vec<_>, Vec<_>) = opens
            .map(|request| (Order::<_, OpenInFlight>::from(&request), request))
            .unzip();

        // Collect remaining order Iterators
        let cancels = cancels.collect::<Vec<_>>();
        let refused_cancels = refused_cancels.collect::<Vec<_>>();
        let refused_opens = refused_opens.collect::<Vec<_>>();

        // Send Risk checked requests
        self.send_approved_orders_for_execution(&cancels, &opens)?;

        // Record RiskApproved InFlight orders
        self.state.orders_mut().record_in_flights(in_flights);

        Ok(AuditEventKindRequests {
            cancels,
            opens,
            refused_cancels,
            refused_opens
        })
    }
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

    fn send_approved_orders_for_execution<Error>(
        &self,
        cancels: &[RiskApproved<Order<InstrumentKey, RequestCancel>>],
        opens: &[RiskApproved<Order<InstrumentKey, RequestOpen>>],
    ) -> Result<(), Error>
    where
        ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = Error>,
        InstrumentKey: Clone,
    {
        if !cancels.is_empty() {
            self.execution_tx.send(ExecutionRequest::from_iter(
                opens.iter().cloned().map(RiskApproved::into_item),
            ))?;
        }

        if !opens.is_empty() {
            self.execution_tx.send(ExecutionRequest::from_iter(
                opens.iter().cloned().map(RiskApproved::into_item),
            ))?;
        }

        Ok(())
    }
}

pub struct EngineRunBuilder<EventFeed, AuditTx, ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey, FnShutdown> {
    feed: Option<EventFeed>,
    auditor: Option<Auditor<AuditTx>>,
    engine: Option<Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>>,
    on_shutdown: Option<FnShutdown>,
}

impl<EventFeed, AuditTx, ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey, FnShutdown, Error>
EngineRunBuilder<EventFeed, AuditTx, ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey, FnShutdown>
where
    FnShutdown: Fn(&mut Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>) -> Result<(), Error>,
{
    pub fn new() -> Self {
        Self {
            feed: None,
            auditor: None,
            engine: None,
            on_shutdown: None,
        }
    }

    pub fn feed(self, feed: EventFeed) -> Self {
        Self {
            feed: Some(feed),
            ..self
        }
    }

    pub fn auditor(self, auditor: Auditor<AuditTx>) -> Self {
        Self {
            auditor: Some(auditor),
            ..self
        }
    }

    pub fn engine(self, engine: Engine<ExecutionTx, State, StrategyT, Risk, AssetKey, InstrumentKey>) -> Self {
        Self {
            engine: Some(engine),
            ..self
        }
    }

    pub fn on_shutdown(self, on_shutdown: FnShutdown) -> Self {
        Self {
            on_shutdown: Some(on_shutdown),
            ..self
        }
    }

    pub fn build(self) {
        todo!()
    }
}