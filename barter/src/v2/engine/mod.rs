use crate::v2::{channel::Tx, engine::{
    audit::{AuditEvent, Auditor},
    state::{instrument::OrderManager, EngineState},
}, execution::ExecutionRequest, instrument::asset::AssetId, order::{OpenInFlight, Order, RequestCancel, RequestOpen}, risk::{RiskApproved, RiskManager}, strategy::Strategy, TryUpdater};
use barter_data::instrument::InstrumentId;
use derive_more::Constructor;
use std::fmt::Debug;


pub mod audit;
pub mod command;
pub mod error;
pub mod state;
pub mod ext;

#[derive(Debug, Constructor)]
pub struct Engine<EventFeed, ExecutionTx, AuditTx, State, StrategyT, Risk> {
    pub feed: EventFeed,
    pub execution_tx: ExecutionTx,
    pub auditor: Auditor<AuditTx>,
    pub state: State,
    pub strategy: StrategyT,
    pub risk: Risk,
}

impl<EventFeed, Event, ExecutionTx, AuditTx, State, StrategyT, Risk, Error>
    Engine<EventFeed, ExecutionTx, AuditTx, State, StrategyT, Risk>
where
    EventFeed: Iterator<Item = Event>,
    Event: Debug,
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentId>, Error = Error>,
    AuditTx: Tx<Item = AuditEvent<State, Event, InstrumentId>, Error = Error>,
    State: EngineState<Event, AssetId, InstrumentId, StrategyT::State, Risk::State>,
    StrategyT: Strategy<State, Event = Event>,
    Risk: RiskManager<State, Event = Event>,
{
    pub fn run_with_shutdown<F>(&mut self, shutdown: F) -> Result<(), Error>
    where
        F: Fn(&mut Self) -> Result<(), Error>,
        Error: for<'a> From<<State as TryUpdater<&'a Event>>::Error>
    {
        self.run()?;
        shutdown(self)
    }

    pub fn run(&mut self) -> Result<(), Error>
    where
        Error: for<'a> From<<State as TryUpdater<&'a Event>>::Error>
    {
        // Send initial EngineState audit snapshot
        self.auditor.audit_snapshot(self.state.clone());

        while let Some(event) = self.feed.next() {
            self.state.try_update(&event)?;

            // Generate orders
            let (cancels, opens) = self.strategy.generate_orders(&self.state);

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

            // Send EngineState audit update
            self.auditor.audit(event, cancels, opens, refused_cancels, refused_opens);
        }

        Ok(())
    }

    fn send_approved_orders_for_execution(
        &self,
        cancels: &[RiskApproved<Order<InstrumentId, RequestCancel>>],
        opens: &[RiskApproved<Order<InstrumentId, RequestOpen>>],
    ) -> Result<(), Error>
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
