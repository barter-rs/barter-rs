use crate::v2::{
    engine::{
        action::{
            generate_algo_orders::GenerateAlgoOrders, send_requests::SendRequestsOutput,
            ActionOutput,
        },
        audit::{Audit, AuditEvent, Auditor},
        command::Command,
        error::{EngineError, RecoverableEngineError, UnrecoverableEngineError},
        execution_tx::ExecutionTxMap,
        state::{
            asset::{manager::AssetStateManager, AssetState},
            connectivity::{manager::ConnectivityManager, Connection, ConnectivityState},
            instrument::{
                manager::{InstrumentFilter, InstrumentStateManager},
                market_data::MarketDataState,
                InstrumentState,
            },
            trading::TradingState,
            EngineState, EngineStateManager, IndexedEngineState,
        },
    },
    execution::{manager::AccountStreamEvent, AccountEvent, ExecutionRequest},
    order::Order,
    risk::{RiskManager, RiskManagerNew},
    strategy::{
        algo::AlgoStrategy,
        close_positions::{ClosePositionsStrategy, ClosePositionsStrategyNew},
        on_disconnect::OnDisconnectStrategy,
        Strategy,
    },
    EngineEvent,
};
use audit::{request::ExecutionRequestAudit, shutdown::ShutdownAudit};
use barter_data::{event::MarketEvent, streams::consumer::MarketStreamEvent};
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    instrument::InstrumentIndex,
};
use barter_integration::{
    channel::{ChannelTxDroppable, Tx},
    collection::none_one_or_many::NoneOneOrMany,
    Unrecoverable,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use std::fmt::Debug;
use tracing::{error, warn};

pub mod action;
pub mod audit;
pub mod command;
pub mod error;
pub mod execution_tx;
pub mod state;

pub trait Processor<Event> {
    type Output;
    fn process(&mut self, event: Event) -> Self::Output;
}

pub fn run<Events, Engine, AuditTx>(
    feed: &mut Events,
    engine: &mut Engine,
    audit_tx: &mut ChannelTxDroppable<AuditTx>,
) -> ShutdownAudit<Events::Item>
where
    Events: Iterator,
    Events::Item: Clone,
    Engine: Processor<Events::Item> + Auditor<Engine::Output>,
    Engine::Output: From<Engine::Snapshot> + From<ShutdownAudit<Events::Item>>,
    AuditTx: Tx<Item = AuditEvent<Engine::Output>>,
{
    // Send initial Engine state snapshot
    audit_tx.send(engine.build_audit(engine.snapshot()));

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(event) = feed.next() else {
            break ShutdownAudit::FeedEnded;
        };

        let audit_kind = engine.process(event);
        audit_tx.send(engine.build_audit(audit_kind));
    };

    // Send Shutdown audit
    audit_tx.send(engine.build_audit(shutdown_audit.clone()));
    shutdown_audit
}

pub type IndexedEngine<
    MarketState,
    ExecutionTxs,
    StrategyT: Strategy,
    Risk: RiskManager<MarketState, ExchangeIndex, AssetIndex, InstrumentIndex>,
> = Engine<IndexedEngineState<MarketState, StrategyT::State, Risk>, ExecutionTxs, StrategyT, Risk>;

// impl<MarketState, ExecutionTxs, StrategyT, Risk> ConnectivityManager<ExchangeIndex> for IndexedEngine<
//     MarketState,
//     ExecutionTxs,
//     StrategyT,
//     Risk
// >
// where
//     StrategyT: Strategy,
//     Risk: RiskManager<MarketState, ExchangeIndex, AssetIndex, InstrumentIndex>
// {
//     fn connectivity(&self, key: &ExchangeIndex) -> &ConnectivityState {
//         self.state
//             .connectivity
//             .0
//             .get_index(key.index())
//             .map(|(_, state)| state)
//             .unwrap()
//     }
//
//     fn connectivity_mut(&mut self, key: &ExchangeIndex) -> &mut ConnectivityState {
//
//     }
// }

pub struct Engine<State, ExecutionTxs, Strategy, Risk> {
    pub sequence: u64,
    pub clock: fn() -> DateTime<Utc>,
    pub state: State,
    pub execution_txs: ExecutionTxs,
    pub strategy: Strategy,
    pub risk: Risk,
}

// pub trait StateManagerNew<ExchangeKey, AssetKey, InstrumentKey> {
//     type MarketState;
//
//     fn connectivity(&self, key: &ExchangeKey) -> &ConnectivityState;
//     fn connectivity_mut(&mut self, key: &ExchangeKey) -> &mut ConnectivityState;
//     fn asset(&self, key: &AssetKey) -> &AssetState;
//     fn asset_mut(&mut self, key: &AssetKey) -> &mut AssetState;
//     fn instrument(
//         &self,
//         key: &InstrumentKey,
//     ) -> &InstrumentState<Self::MarketState, ExchangeKey, AssetKey, InstrumentKey>;
//
//     fn instrument_mut(
//         &mut self,
//         key: &InstrumentKey,
//     ) -> &mut InstrumentState<Self::MarketState, ExchangeKey, AssetKey, InstrumentKey>;
//
//     fn instruments<'a>(
//         &'a self,
//         _filter: &'a InstrumentFilter<ExchangeKey, AssetKey, InstrumentKey>,
//     ) -> impl Iterator<
//         Item = &'a InstrumentState<Self::MarketState, ExchangeKey, AssetKey, InstrumentKey>,
//     >
//     where
//         Self::MarketState: 'a,
//         ExchangeKey: 'a,
//         AssetKey: 'a,
//         InstrumentKey: 'a;
// }

impl<State, ExecutionTxs, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
    Processor<EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>>
    for Engine<State, ExecutionTxs, Strategy, Risk>
where
    State: EngineStateManager<ExchangeKey, AssetKey, InstrumentKey>,
    ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
    // Strategy: AlgoStrategy<MarketState, ExchangeKey, AssetKey, InstrumentKey>
{
    type Output = Audit<
        State,
        EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>,
        ExecutionRequestAudit<ExchangeKey, InstrumentKey>,
    >;

    fn process(
        &mut self,
        event: EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> Self::Output {
        match &event {
            EngineEvent::Shutdown => return Audit::Shutdown(ShutdownAudit::Commanded(event)),
            EngineEvent::Command(command) => {
                let output = self.action(command);

                return if let Some(unrecoverable) = output.unrecoverable_errors() {
                    Audit::ShutdownWithOutput(ShutdownAudit::Error(event, unrecoverable), output)
                } else {
                    Audit::ProcessWithOutput(event, output)
                };
            }
            EngineEvent::TradingStateUpdate(trading_state) => {
                self.state.update_from_trading_state_update(trading_state);
            }
            EngineEvent::Account(account) => {
                self.update_from_account(account);
            }
            EngineEvent::Market(market) => {
                self.update_from_market(market);
            }
        };

        if let TradingState::Enabled = self.state.trading {
            let output = self.generate_algo_orders();

            if let Some(unrecoverable) = output.unrecoverable_errors() {
                Audit::ShutdownWithOutput(ShutdownAudit::Error(event, unrecoverable), output)
            } else {
                Audit::ProcessWithOutput(event, output)
            }
        } else {
            Audit::Process(event)
        }
    }
}

impl<State, ExecutionTxs, Strategy, Risk> Engine<State, ExecutionTxs, Strategy, Risk> {
    pub fn action<ExchangeKey, AssetKey, InstrumentKey>(
        &mut self,
        command: &Command<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> ActionOutput<ExchangeKey, InstrumentKey>
    where
        State: EngineStateManager<ExchangeKey, AssetKey, InstrumentKey>,
        ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
        Strategy: ClosePositionsStrategyNew<ExchangeKey, AssetKey, InstrumentKey>,
        Risk: RiskManagerNew<ExchangeKey, AssetKey, InstrumentKey>,
        ExchangeKey: Debug + Clone,
        InstrumentKey: Debug + Clone,
    {
        match &command {
            Command::SendCancelRequests(requests) => {
                let output = self.send_requests(requests.clone());
                self.state.record_in_flight_cancels(&output.sent);
                ActionOutput::CancelOrders(output)
            }
            Command::SendOpenRequests(requests) => {
                let output = self.send_requests(requests.clone());
                self.state.record_in_flight_opens(&output.sent);
                ActionOutput::OpenOrders(output)
            }
            Command::ClosePositions(filter) => {
                ActionOutput::ClosePositions(self.close_positions(filter))
            }
            Command::CancelOrders(filter) => ActionOutput::CancelOrders(self.cancel_orders(filter)),
        }
    }

    pub fn send_requests<ExchangeKey, InstrumentKey, Kind>(
        &self,
        requests: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, Kind>>,
    ) -> SendRequestsOutput<ExchangeKey, InstrumentKey, Kind>
    where
        ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
        ExchangeKey: Debug + Clone,
        InstrumentKey: Debug + Clone,
        Kind: Debug + Clone,
        ExecutionRequest<ExchangeKey, InstrumentKey>: From<Order<ExchangeKey, InstrumentKey, Kind>>,
    {
        // Send order requests
        let (sent, errors): (Vec<_>, Vec<_>) = requests
            .into_iter()
            .map(|request| {
                self.send_request(&request)
                    .map_err(|error| (request.clone(), error))
                    .map(|_| request)
            })
            .partition_result();

        SendRequestsOutput::new(NoneOneOrMany::from(sent), NoneOneOrMany::from(errors))
    }

    pub fn send_request<ExchangeKey, InstrumentKey, Kind>(
        &self,
        request: &Order<ExchangeKey, InstrumentKey, Kind>,
    ) -> Result<(), EngineError>
    where
        ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
        ExchangeKey: Debug + Clone,
        InstrumentKey: Debug + Clone,
        Kind: Debug + Clone,
        ExecutionRequest<ExchangeKey, InstrumentKey>: From<Order<ExchangeKey, InstrumentKey, Kind>>,
    {
        match self
            .execution_txs
            .find(&request.exchange)?
            .send(ExecutionRequest::from(request.clone()))
        {
            Ok(()) => Ok(()),
            Err(error) if error.is_unrecoverable() => {
                error!(
                    exchange = ?request.exchange,
                    ?request,
                    ?error,
                    "failed to send ExecutionRequest due to terminated channel"
                );
                Err(EngineError::Unrecoverable(
                    UnrecoverableEngineError::ExecutionChannelTerminated(format!(
                        "{:?} execution channel terminated: {:?}",
                        request.exchange, error
                    )),
                ))
            }
            Err(error) => {
                error!(
                    exchange = ?request.exchange,
                    ?request,
                    ?error,
                    "failed to send ExecutionRequest due to unhealthy channel"
                );
                Err(EngineError::Recoverable(
                    RecoverableEngineError::ExecutionChannelUnhealthy(format!(
                        "{:?} execution channel unhealthy: {:?}",
                        request.exchange, error
                    )),
                ))
            }
        }
    }
}

impl<ExecutionTxs, MarketState, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
    Processor<EngineEvent<MarketState::EventKind, ExchangeKey, AssetKey, InstrumentKey>>
    for Engine<
        EngineState<
            MarketState,
            Strategy::State,
            Risk::State,
            ExchangeKey,
            AssetKey,
            InstrumentKey,
        >,
        ExecutionTxs,
        Strategy,
        Risk,
    >
where
    ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
    MarketState: MarketDataState<InstrumentKey>,
    Strategy: AlgoStrategy<MarketState, ExchangeKey, AssetKey, InstrumentKey>
        + ClosePositionsStrategy<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    Strategy::State: for<'a> Processor<&'a AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>
        + for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketState::EventKind>>,
    Risk: RiskManager<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    Risk::State: for<'a> Processor<&'a AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>
        + for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketState::EventKind>>,
    ExchangeKey: Debug + Clone + PartialEq,
    AssetKey: Debug,
    InstrumentKey: Debug + Clone + PartialEq,
    EngineState<MarketState, Strategy::State, Risk::State, ExchangeKey, AssetKey, InstrumentKey>:
        ConnectivityManager<ExchangeId>
            + AssetStateManager<AssetKey>
            + InstrumentStateManager<InstrumentKey, ExchangeKey = ExchangeKey, AssetKey = AssetKey>,
{
    type Output = Audit<
        EngineState<
            MarketState,
            Strategy::State,
            Risk::State,
            ExchangeKey,
            AssetKey,
            InstrumentKey,
        >,
        EngineEvent<MarketState::EventKind, ExchangeKey, AssetKey, InstrumentKey>,
        ExecutionRequestAudit<ExchangeKey, InstrumentKey>,
    >;

    fn process(
        &mut self,
        event: EngineEvent<MarketState::EventKind, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> Self::Output {
        match &event {
            EngineEvent::Shutdown => return Audit::Shutdown(ShutdownAudit::Commanded(event)),
            EngineEvent::Command(command) => {
                let output = self.action(command);

                return if let Some(unrecoverable) = output.unrecoverable_errors() {
                    Audit::ShutdownWithOutput(ShutdownAudit::Error(event, unrecoverable), output)
                } else {
                    Audit::ProcessWithOutput(event, output)
                };
            }
            EngineEvent::TradingStateUpdate(trading_state) => {
                self.state.update_from_trading_state_update(trading_state);
            }
            EngineEvent::Account(account) => {
                self.update_from_account(account);
            }
            EngineEvent::Market(market) => {
                self.update_from_market(market);
            }
        };

        if let TradingState::Enabled = self.state.trading {
            let output = self.generate_algo_orders();

            if let Some(unrecoverable) = output.unrecoverable_errors() {
                Audit::ShutdownWithOutput(ShutdownAudit::Error(event, unrecoverable), output)
            } else {
                Audit::ProcessWithOutput(event, output)
            }
        } else {
            Audit::Process(event)
        }
    }
}

impl<MarketState, ExecutionTxs, Strategy, Risk>
    Engine<IndexedEngineState<MarketState, Strategy, Risk>, ExecutionTxs, Strategy, Risk>
{
}

impl<MarketState, ExecutionTxs, StrategyT, Risk, ExchangeKey, AssetKey, InstrumentKey>
    Engine<
        EngineState<
            MarketState,
            StrategyT::State,
            Risk::State,
            ExchangeKey,
            AssetKey,
            InstrumentKey,
        >,
        ExecutionTxs,
        StrategyT,
        Risk,
    >
where
    MarketState: MarketDataState<InstrumentKey>,
    StrategyT: OnDisconnectStrategy,
    Risk: RiskManager<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
    EngineState<MarketState, StrategyT::State, Risk::State, ExchangeKey, AssetKey, InstrumentKey>:,
{
    pub fn update_from_account(
        &mut self,
        event: &AccountStreamEvent<ExchangeKey, AssetKey, InstrumentKey>,
    ) {
        match event {
            AccountStreamEvent::Reconnecting(exchange) => {
                warn!(%exchange, "EngineState received AccountStream disconnected event");
                self.state.connectivity_mut(exchange).account = Connection::Reconnecting;
                let x = Strategy::on_disconnect(self, *exchange);
            }
            AccountStreamEvent::Item(event) => {
                let x = self.state.process(event);
            }
        }
    }

    pub fn update_from_market(
        &mut self,
        event: &MarketStreamEvent<InstrumentKey, MarketState::EventKind>,
    ) {
    }
}

impl<State, ExecutionTxs, Strategy, Risk> Engine<State, ExecutionTxs, Strategy, Risk> {
    pub fn new(
        clock: fn() -> DateTime<Utc>,
        state: State,
        execution_txs: ExecutionTxs,
        strategy: Strategy,
        risk: Risk,
    ) -> Self {
        Self {
            sequence: 0,
            clock,
            state,
            execution_txs,
            strategy,
            risk,
        }
    }

    pub fn sequence_fetch_add(&mut self) -> u64 {
        let sequence = self.sequence;
        self.sequence += 1;
        sequence
    }
}

impl<AuditKind, State, ExecutionTx, StrategyT, Risk> Auditor<AuditKind>
    for Engine<State, ExecutionTx, StrategyT, Risk>
where
    AuditKind: From<State>,
    State: Clone,
{
    type Snapshot = State;

    fn snapshot(&self) -> Self::Snapshot {
        self.state.clone()
    }

    fn build_audit<Kind>(&mut self, kind: Kind) -> AuditEvent<AuditKind>
    where
        AuditKind: From<Kind>,
    {
        AuditEvent {
            id: self.sequence_fetch_add(),
            time: (self.clock)(),
            kind: AuditKind::from(kind),
        }
    }
}

// // Todo: could further abstract State here...
// impl<ExecutionTxs, MarketState, StrategyT, Risk, ExchangeKey, AssetKey, InstrumentKey>
//     Engine<
//         EngineState<
//             MarketState,
//             StrategyT::State,
//             Risk::State,
//             ExchangeKey,
//             AssetKey,
//             InstrumentKey,
//         >,
//         ExecutionTxs,
//         StrategyT,
//         Risk,
//     >
// where
//     ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
//     MarketState: MarketDataState<InstrumentKey>,
//     StrategyT: Strategy<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
//     Risk: RiskManager<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
//     ExchangeKey: Debug + Clone,
//     InstrumentKey: Debug + Clone,
//     EngineState<MarketState, StrategyT::State, Risk::State, ExchangeKey, AssetKey, InstrumentKey>:
//         StateManager<AssetKey, State = AssetState>
//             + StateManager<
//                 InstrumentKey,
//                 State = InstrumentState<MarketState, ExchangeKey, AssetKey, InstrumentKey>,
//             >,
// {
//     pub fn action(
//         &mut self,
//         command: &Command<ExchangeKey, InstrumentKey>,
//     ) -> ExecutionRequestAudit<ExchangeKey, InstrumentKey> {
//         let sent_audit = match command {
//             Command::Execute(ExecutionRequest::Cancel(request)) => {
//                 self.action_execute_cancel(request)
//             }
//             Command::Execute(ExecutionRequest::Open(request)) => self.action_execute_open(request),
//             Command::ClosePosition(instrument) => self.close_position(instrument),
//             Command::CloseAllPositions => self.close_all_positions(),
//             Command::CancelOrderById((instrument, id)) => self.cancel_order_by_id(instrument, id),
//             Command::CancelAllOrders => self.cancel_all_orders(),
//         };
//
//         ExecutionRequestAudit::from(sent_audit)
//     }
//
//     pub fn action_execute_cancel(
//         &mut self,
//         request: &Order<ExchangeKey, InstrumentKey, RequestCancel>,
//     ) -> SentRequestsAudit<ExchangeKey, InstrumentKey> {
//         self.send_execution_request(request)
//             .map(|_| SentRequestsAudit {
//                 cancels: NoneOneOrMany::One(request.clone()),
//                 ..Default::default()
//             })
//             .unwrap_or_else(|error| SentRequestsAudit {
//                 failed_cancels: NoneOneOrMany::One((request.clone(), EngineError::from(error))),
//                 ..Default::default()
//             })
//     }
//
//     pub fn send_execution_request<Kind>(
//         &self,
//         request: &Order<ExchangeKey, InstrumentKey, Kind>,
//     ) -> Result<(), UnrecoverableEngineError>
//     where
//         Kind: Clone + Debug,
//         ExecutionRequest<ExchangeKey, InstrumentKey>: From<Order<ExchangeKey, InstrumentKey, Kind>>,
//     {
//         match self
//             .execution_txs
//             .find(&request.exchange)?
//             .send(ExecutionRequest::from(request.clone()))
//         {
//             Ok(()) => Ok(()),
//             Err(error) if error.is_unrecoverable() => {
//                 Err(UnrecoverableEngineError::ExecutionChannelTerminated(
//                     format!(
//                         "{:?} execution channel terminated, failed to send {:?}",
//                         request.exchange, request
//                     )
//                     .to_string(),
//                 ))
//             }
//             Err(error) => {
//                 error!(
//                     exchange = ?request.exchange,
//                     ?request,
//                     ?error,
//                     "failed to send ExecutionRequest due to unhealthy channel"
//                 );
//                 Ok(())
//             }
//         }
//     }
//
//     pub fn action_execute_open(
//         &mut self,
//         request: &Order<ExchangeKey, InstrumentKey, RequestOpen>,
//     ) -> SentRequestsAudit<ExchangeKey, InstrumentKey> {
//         self.send_execution_request(request)
//             .map(|_| SentRequestsAudit {
//                 opens: NoneOneOrMany::One(request.clone()),
//                 ..Default::default()
//             })
//             .unwrap_or_else(|error| SentRequestsAudit {
//                 failed_opens: NoneOneOrMany::One((request.clone(), EngineError::from(error))),
//                 ..Default::default()
//             })
//     }
//
//     pub fn close_position(
//         &mut self,
//         instrument: &InstrumentKey,
//     ) -> SentRequestsAudit<ExchangeKey, InstrumentKey> {
//         // Generate orders
//         let (cancels, opens) = self.strategy.close_position_request(
//             instrument,
//             &self.state.strategy,
//             &self.state.assets,
//             &self.state.instruments,
//         );
//
//         // Bypass risk checks...
//
//         self.send_orders(cancels, opens)
//     }
//
//     pub fn close_all_positions(&mut self) -> SentRequestsAudit<ExchangeKey, InstrumentKey> {
//         // Generate orders
//         let (cancels, opens) = self.strategy.close_all_positions_request(
//             &self.state.strategy,
//             &self.state.assets,
//             &self.state.instruments,
//         );
//
//         // Bypass risk checks...
//
//         self.send_orders(cancels, opens)
//     }
//
//     pub fn cancel_order_by_id(
//         &mut self,
//         _instrument: &InstrumentKey,
//         _id: &OrderId,
//     ) -> SentRequestsAudit<ExchangeKey, InstrumentKey> {
//         // Todo: this evenings plan:
//         //  1. Implement CancelAllOrders & CancelOrderById
//         //  2. Re-design ExecutionManager to run request futures concurrently
//
//         // Todo: Open Q:
//         // - Maybe CancelAllOrders, etc, should only be accessible via Command to keep audit flow
//         //   in tact?
//
//         todo!()
//         // self.execution_tx.send(ExecutionRequest::CancelOrder(RequestCancel::new(instrument, id)))
//     }
//
//     pub fn cancel_all_orders(&mut self) -> SentRequestsAudit<ExchangeKey, InstrumentKey> {
//         todo!()
//     }
//
//     pub fn trade(&mut self) -> ExecutionRequestAudit<ExchangeKey, InstrumentKey> {
//         // Generate orders
//         let (cancels, opens) = self.strategy.generate_orders(
//             &self.state.strategy,
//             &self.state.assets,
//             &self.state.instruments,
//         );
//
//         // RiskApprove & RiskRefuse order requests
//         let (cancels, opens, refused_cancels, refused_opens) = self.risk.check(
//             &self.state.risk,
//             &self.state.assets,
//             &self.state.instruments,
//             cancels,
//             opens,
//         );
//
//         // Send risk approved order requests
//         let sent = self.send_orders(
//             cancels.into_iter().map(|RiskApproved(cancel)| cancel),
//             opens.into_iter().map(|RiskApproved(open)| open),
//         );
//
//         // Collect remaining Iterators (so we can &mut self)
//         let refused = RiskRefusedRequestsAudit {
//             refused_cancels: refused_cancels.into_iter().collect(),
//             refused_opens: refused_opens.into_iter().collect(),
//         };
//
//         // Record in flight order requests
//         self.record_requests_in_flight(&sent.cancels, &sent.opens);
//
//         ExecutionRequestAudit { sent, refused }
//     }
//
//     pub fn send_orders(
//         &self,
//         cancels: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestCancel>>,
//         opens: impl IntoIterator<Item = Order<ExchangeKey, InstrumentKey, RequestOpen>>,
//     ) -> SentRequestsAudit<ExchangeKey, InstrumentKey> {
//         // Send order requests
//         let (cancels_sent, cancel_send_errs): (Vec<_>, Vec<_>) = cancels
//             .into_iter()
//             .map(|cancel| {
//                 self.send_execution_request(&cancel)
//                     .map_err(|error| (cancel.clone(), EngineError::from(error)))
//                     .map(|_| cancel)
//             })
//             .partition_result();
//
//         let (opens_sent, open_send_errs): (Vec<_>, Vec<_>) = opens
//             .into_iter()
//             .map(|open| {
//                 self.send_execution_request(&open)
//                     .map_err(|error| (open.clone(), EngineError::from(error)))
//                     .map(|_| open)
//             })
//             .partition_result();
//
//         SentRequestsAudit {
//             cancels: NoneOneOrMany::Many(cancels_sent),
//             opens: NoneOneOrMany::Many(opens_sent),
//             failed_cancels: NoneOneOrMany::from(cancel_send_errs),
//             failed_opens: NoneOneOrMany::from(open_send_errs),
//         }
//     }
//
//     pub fn record_requests_in_flight<'a>(
//         &mut self,
//         cancels: impl IntoIterator<Item = &'a Order<ExchangeKey, InstrumentKey, RequestCancel>>,
//         opens: impl IntoIterator<Item = &'a Order<ExchangeKey, InstrumentKey, RequestOpen>>,
//     ) where
//         ExchangeKey: 'a,
//         InstrumentKey: 'a,
//     {
//         for request in cancels.into_iter() {
//             self.record_request_cancel_in_flight(request);
//         }
//         for request in opens.into_iter() {
//             self.record_request_open_in_flight(request);
//         }
//     }
//
//     pub fn record_request_cancel_in_flight(
//         &mut self,
//         request: &Order<ExchangeKey, InstrumentKey, RequestCancel>,
//     ) {
//         self.state
//             .state_mut(&request.instrument)
//             .orders
//             .record_in_flight_cancel(request);
//     }
//
//     pub fn record_request_open_in_flight(
//         &mut self,
//         request: &Order<ExchangeKey, InstrumentKey, RequestOpen>,
//     ) {
//         self.state
//             .state_mut(&request.instrument)
//             .orders
//             .record_in_flight_open(request);
//     }
// }
