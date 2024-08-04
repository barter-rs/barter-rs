use crate::v2::{
    channel::Tx,
    engine::{
        audit::{AuditEvent, Auditor},
        error::EngineError,
        state::{instrument::OrderManager, EngineState},
    },
    execution::ExecutionRequest,
    instrument::asset::AssetId,
    order::{OpenInFlight, Order, RequestCancel, RequestOpen},
    risk::{RiskApproved, RiskManager},
    strategy::Strategy,
};
use barter_data::instrument::InstrumentId;
use derive_more::Constructor;
use std::fmt::Debug;
use tracing::error;

pub mod audit;
pub mod command;
pub mod error;
pub mod state;

#[derive(Debug, Constructor)]
pub struct Engine<EventFeed, Event, ExecutionTx, AuditTx, State, StrategyT, Risk>
where
    State: EngineState<Event, AssetId, InstrumentId, StrategyT::State, Risk::State>,
    StrategyT: Strategy<State, Event = Event>,
    Risk: RiskManager<State, Event = Event>,
{
    pub feed: EventFeed,
    pub execution_tx: ExecutionTx,
    pub auditor: Auditor<AuditTx>,
    pub state: State,
    pub strategy: StrategyT,
    pub risk: Risk,
}

impl<EventFeed, Event, ExecutionTx, AuditTx, State, StrategyT, Risk>
    Engine<EventFeed, Event, ExecutionTx, AuditTx, State, StrategyT, Risk>
where
    EventFeed: Iterator<Item = Event>,
    Event: Debug,
    ExecutionTx: Tx<Item = ExecutionRequest<InstrumentId>, Error = EngineError>,
    AuditTx: Tx<Item = AuditEvent<State, Event, InstrumentId>, Error = EngineError>,
    State: EngineState<Event, AssetId, InstrumentId, StrategyT::State, Risk::State>,
    StrategyT: Strategy<State, Event = Event>,
    Risk: RiskManager<State, Event = Event>,
{
    pub fn run(self) -> State {
        let Self {
            feed,
            execution_tx,
            mut auditor,
            mut state,
            strategy,
            risk,
        } = self;

        // Send initial EngineState audit snapshot
        auditor.audit_snapshot(state.clone());

        for event in feed {
            // Update State
            if let Err(error) = state.try_update(&event) {
                error!(?error, "terminating Engine");
                break;
            }

            // Generate orders
            let (cancels, opens) = strategy.generate_orders(&state);

            // RiskApprove & RiskRefuse orders
            let (cancels, opens, refused_cancels, refused_opens) =
                risk.approve_orders(&state, cancels, opens);

            // Generate InFlights for RiskApproved orders
            let (in_flights, opens): (Vec<_>, Vec<_>) = opens
                .map(|request| (Order::<_, OpenInFlight>::from(&request), request))
                .unzip();

            // Collect remaining order Iterators
            let cancels = cancels.collect::<Vec<_>>();
            let refused_cancels = refused_cancels.collect::<Vec<_>>();
            let refused_opens = refused_opens.collect::<Vec<_>>();

            if Self::send_approved_orders_for_execution(&execution_tx, &cancels, &opens).is_err() {
                error!(error = "ExecutionRx dropped", "terminating Engine");
                break;
            }

            // Record RiskApproved InFlight orders
            state.orders_mut().record_in_flights(in_flights);

            // Send EngineState audit update
            auditor.audit(event, cancels, opens, refused_cancels, refused_opens);
        }

        // Return State for post-trading analysis
        state
    }

    fn send_approved_orders_for_execution(
        execution_tx: &ExecutionTx,
        cancels: &[RiskApproved<Order<InstrumentId, RequestCancel>>],
        opens: &[RiskApproved<Order<InstrumentId, RequestOpen>>],
    ) -> Result<(), EngineError> {
        if !cancels.is_empty() {
            execution_tx.send(ExecutionRequest::from_iter(
                opens.iter().cloned().map(RiskApproved::into_item),
            ))?;
        }

        if !opens.is_empty() {
            execution_tx.send(ExecutionRequest::from_iter(
                opens.iter().cloned().map(RiskApproved::into_item),
            ))?;
        }

        Ok(())
    }
}
