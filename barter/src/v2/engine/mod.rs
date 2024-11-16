use crate::v2::{
    engine::{
        action::{
            cancel_orders::CancelOrders,
            close_positions::ClosePositions,
            generate_algo_orders::{GenerateAlgoOrders, GenerateAlgoOrdersOutput},
            send_requests::SendRequests,
            ActionOutput,
        },
        audit::{Audit, AuditEvent, Auditor},
        command::Command,
        execution_tx::ExecutionTxMap,
        state::{
            connectivity::Connection,
            instrument::manager::InstrumentStateManager,
            order::in_flight_recorder::InFlightRequestRecorder,
            trading::{manager::TradingStateManager, TradingState},
            StateManager,
        },
    },
    execution::manager::AccountStreamEvent,
    risk::RiskManager,
    strategy::{
        algo::AlgoStrategy, close_positions::ClosePositionsStrategy,
        on_disconnect::OnDisconnectStrategy, on_trading_disabled::OnTradingDisabled,
    },
    EngineEvent,
};
use audit::shutdown::ShutdownAudit;
use barter_data::streams::consumer::MarketStreamEvent;
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};
use barter_integration::channel::{ChannelTxDroppable, Tx};
use chrono::{DateTime, Utc};
use derive_more::From;
use std::fmt::Debug;
use tracing::warn;

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

pub struct Engine<State, ExecutionTxs, Strategy, Risk> {
    pub sequence: u64,
    pub clock: fn() -> DateTime<Utc>,
    pub state: State,
    pub execution_txs: ExecutionTxs,
    pub strategy: Strategy,
    pub risk: Risk,
}

type IndexedEngineOutput<OnTradingDisabled, OnDisconnect> =
    EngineOutput<ExchangeIndex, InstrumentIndex, OnTradingDisabled, OnDisconnect>;

#[derive(Debug, Clone)]
pub enum EngineOutput<ExchangeKey, InstrumentKey, OnTradingDisabled, OnDisconnect> {
    Commanded(ActionOutput<ExchangeKey, InstrumentKey>),
    OnTradingDisabled(OnTradingDisabled),
    OnDisconnect(OnDisconnect),
    AlgoOrders(GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>),
}

impl<ExchangeKey, InstrumentKey, OnTradingDisabled, OnDisconnect>
    From<ActionOutput<ExchangeKey, InstrumentKey>>
    for EngineOutput<ExchangeKey, InstrumentKey, OnTradingDisabled, OnDisconnect>
{
    fn from(value: ActionOutput<ExchangeKey, InstrumentKey>) -> Self {
        Self::Commanded(value)
    }
}

impl<ExchangeKey, InstrumentKey, OnTradingDisabled, OnDisconnect>
    From<GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>>
    for EngineOutput<ExchangeKey, InstrumentKey, OnTradingDisabled, OnDisconnect>
{
    fn from(value: GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>) -> Self {
        Self::AlgoOrders(value)
    }
}

impl<State, ExecutionTxs, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
    Processor<EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>>
    for Engine<State, ExecutionTxs, Strategy, Risk>
where
    State: StateManager<ExchangeKey, AssetKey, InstrumentKey>
        + InFlightRequestRecorder<ExchangeKey, InstrumentKey>,
    ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
    Strategy: OnTradingDisabled<State, ExecutionTxs, Risk>
        + OnDisconnectStrategy<State, ExecutionTxs, Risk>
        + AlgoStrategy<ExchangeKey, InstrumentKey, State = State>
        + ClosePositionsStrategy<ExchangeKey, AssetKey, InstrumentKey, State = State>,
    Risk: RiskManager<ExchangeKey, InstrumentKey, State = State>,
    ExchangeKey: Debug + Clone + PartialEq,
    AssetKey: PartialEq,
    InstrumentKey: Debug + Clone + PartialEq,
{
    type Output = Audit<
        State,
        EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>,
        EngineOutput<
            ExchangeKey,
            InstrumentKey,
            Strategy::OnTradingDisabled,
            Strategy::OnDisconnect,
        >,
    >;

    fn process(
        &mut self,
        event: EngineEvent<State::MarketEventKind, ExchangeKey, AssetKey, InstrumentKey>,
    ) -> Self::Output {
        match &event {
            EngineEvent::Shutdown => return Audit::shutdown_commanded(event),
            EngineEvent::Command(command) => {
                let output = self.action(command);

                return if let Some(unrecoverable) = output.unrecoverable_errors() {
                    Audit::shutdown_on_err_with_output(event, unrecoverable, output)
                } else {
                    Audit::process_with_output(event, output)
                };
            }
            EngineEvent::TradingStateUpdate(trading_state) => {
                if let Some(trading_disabled) =
                    self.update_from_trading_state_update(*trading_state)
                {
                    return Audit::process_with_output(
                        event,
                        EngineOutput::OnTradingDisabled(trading_disabled),
                    );
                }
            }
            EngineEvent::Account(account) => {
                if let Some(disconnected) = self.update_from_account_stream(account) {
                    return Audit::process_with_output(
                        event,
                        EngineOutput::OnDisconnect(disconnected),
                    );
                }
            }
            EngineEvent::Market(market) => {
                if let Some(disconnected) = self.update_from_market_stream(market) {
                    return Audit::process_with_output(
                        event,
                        EngineOutput::OnDisconnect(disconnected),
                    );
                }
            }
        };

        if let TradingState::Enabled = self.state.trading() {
            let output = self.generate_algo_orders();
            if let Some(unrecoverable) = output.unrecoverable_errors() {
                Audit::shutdown_on_err_with_output(event, unrecoverable, output)
            } else {
                Audit::process_with_output(event, output)
            }
        } else {
            Audit::process(event)
        }
    }
}

impl<State, ExecutionTxs, Strategy, Risk> Engine<State, ExecutionTxs, Strategy, Risk> {
    pub fn action<ExchangeKey, AssetKey, InstrumentKey>(
        &mut self,
        command: &Command<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> ActionOutput<ExchangeKey, InstrumentKey>
    where
        State: InstrumentStateManager<InstrumentKey, ExchangeKey = ExchangeKey, AssetKey = AssetKey>
            + InFlightRequestRecorder<ExchangeKey, InstrumentKey>,
        ExecutionTxs: ExecutionTxMap<ExchangeKey, InstrumentKey>,
        Strategy: ClosePositionsStrategy<ExchangeKey, AssetKey, InstrumentKey, State = State>,
        Risk: RiskManager<ExchangeKey, InstrumentKey>,
        ExchangeKey: Debug + Clone + PartialEq,
        AssetKey: PartialEq,
        InstrumentKey: Debug + Clone + PartialEq,
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

    pub fn update_from_trading_state_update(
        &mut self,
        update: TradingState,
    ) -> Option<Strategy::OnTradingDisabled>
    where
        State: TradingStateManager,
        Strategy: OnTradingDisabled<State, ExecutionTxs, Risk>,
    {
        // Todo: return Audit too?
        self.state
            .update_trading_state(update)
            .transitioned_to_disabled()
            .then(|| Strategy::on_trading_disabled(self))
    }

    pub fn update_from_account_stream<ExchangeKey, AssetKey, InstrumentKey>(
        &mut self,
        event: &AccountStreamEvent<ExchangeKey, AssetKey, InstrumentKey>,
    ) -> Option<Strategy::OnDisconnect>
    where
        State: StateManager<ExchangeKey, AssetKey, InstrumentKey>,
        Strategy: OnDisconnectStrategy<State, ExecutionTxs, Risk>,
    {
        match event {
            AccountStreamEvent::Reconnecting(exchange) => {
                warn!(%exchange, "EngineState received AccountStream disconnected event");
                self.state.connectivity_mut(exchange).account = Connection::Reconnecting;
                Some(Strategy::on_disconnect(self, *exchange))
            }
            AccountStreamEvent::Item(event) => {
                self.state.update_from_account(event);
                None
            }
        }
    }

    pub fn update_from_market_stream<ExchangeKey, AssetKey, InstrumentKey>(
        &mut self,
        event: &MarketStreamEvent<InstrumentKey, State::MarketEventKind>,
    ) -> Option<Strategy::OnDisconnect>
    where
        State: StateManager<ExchangeKey, AssetKey, InstrumentKey>,
        Strategy: OnDisconnectStrategy<State, ExecutionTxs, Risk>,
    {
        match event {
            MarketStreamEvent::Reconnecting(exchange) => {
                warn!(%exchange, "EngineState received MarketStream disconnect event");
                self.state.connectivity_mut(exchange).market_data = Connection::Reconnecting;
                Some(Strategy::on_disconnect(self, *exchange))
            }
            MarketStreamEvent::Item(event) => {
                self.state.update_from_market(event);
                None
            }
        }
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
