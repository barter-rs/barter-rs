use crate::{
    EngineEvent, Sequence,
    engine::{
        action::{
            ActionOutput,
            cancel_orders::CancelOrders,
            close_positions::ClosePositions,
            generate_algo_orders::{GenerateAlgoOrders, GenerateAlgoOrdersOutput},
            send_requests::SendRequests,
        },
        audit::{AuditTick, Auditor, context::EngineContext},
        clock::EngineClock,
        command::Command,
        execution_tx::ExecutionTxMap,
        state::{
            EngineState, instrument::data::InstrumentDataState,
            order::in_flight_recorder::InFlightRequestRecorder, position::PositionExited,
            trading::TradingState,
        },
    },
    execution::{AccountStreamEvent, request::ExecutionRequest},
    risk::RiskManager,
    shutdown::SyncShutdown,
    statistic::summary::TradingSummaryGenerator,
    strategy::{
        algo::AlgoStrategy, close_positions::ClosePositionsStrategy,
        on_disconnect::OnDisconnectStrategy, on_trading_disabled::OnTradingDisabled,
    },
};
use barter_data::{event::MarketEvent, streams::consumer::MarketStreamEvent};
use barter_execution::AccountEvent;
use barter_instrument::{asset::QuoteAsset, exchange::ExchangeIndex, instrument::InstrumentIndex};
use barter_integration::{Terminal, channel::Tx};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::info;

/// Defines how the [`Engine`] actions a [`Command`], and the associated outputs.
pub mod action;

/// Defines an `Engine` audit types as well as utilities for handling the `Engine` `AuditStream`.
///
/// eg/ `StateReplicaManager` component can be used to maintain an `EngineState` replica.
pub mod audit;

/// Defines the [`EngineClock`] interface used to determine the current `Engine` time.
///
/// This flexibility enables back-testing runs to use approximately correct historical timestamps.
pub mod clock;

/// Defines an [`Engine`] [`Command`] - used to give trading directives to the `Engine` from an
/// external process (eg/ ClosePositions).
pub mod command;

/// Defines all possible errors that can occur in the [`Engine`].
pub mod error;

/// Defines the [`ExecutionTxMap`] interface that models a collection of transmitters used to route
/// can `ExecutionRequest` to the appropriate `ExecutionManagers`.
pub mod execution_tx;

/// Defines all state used by the`Engine` to algorithmically trade.
///
/// eg/ `ConnectivityStates`, `AssetStates`, `InstrumentStates`, `Position`, etc.
pub mod state;

/// `Engine` runners for processing input `Events`.
///
/// eg/ `fn sync_run`, `fn sync_run_with_audit`, `fn async_run`, `fn async_run_with_audit`,
pub mod run;

/// Defines how a component processes an Input and generates an Output.
pub trait Processor<Input> {
    type Output;
    fn process(&mut self, input: Input) -> Self::Output;
}

/// Process and `Event` with the `Engine` and product an [`AuditTick`] of work done.
pub fn process_with_audit<Event, Engine>(
    engine: &mut Engine,
    event: Event,
) -> AuditTick<Engine::Output, EngineContext>
where
    Engine: Processor<Event> + Auditor<Engine::Output, Context = EngineContext>,
{
    let output = engine.process(event);
    engine.audit(output)
}

/// Algorithmic trading `Engine`.
///
/// The `Engine`:
/// * Processes input [`EngineEvent`] (or custom events if implemented).
/// * Maintains the internal [`EngineState`] (instrument data state, open orders, positions, etc.).
/// * Generates algo orders (if `TradingState::Enabled`).
///
/// # Type Parameters
/// * `Clock` - [`EngineClock`] implementation.
/// * `State` - Engine `State` implementation (eg/ [`EngineState`]).
/// * `ExecutionTxs` - [`ExecutionTxMap`] implementation for sending execution requests.
/// * `Strategy` - Trading Strategy implementation (see [`super::strategy`]).
/// * `Risk` - [`RiskManager`] implementation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Engine<Clock, State, ExecutionTxs, Strategy, Risk> {
    pub clock: Clock,
    pub meta: EngineMeta,
    pub state: State,
    pub execution_txs: ExecutionTxs,
    pub strategy: Strategy,
    pub risk: Risk,
}

/// Running [`Engine`] metadata.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct EngineMeta {
    /// [`EngineClock`] start timestamp of the current [`Engine`] `run`.
    pub time_start: DateTime<Utc>,
    /// Monotonically increasing [`Sequence`] associated with the number of events processed.
    pub sequence: Sequence,
}

impl<Clock, GlobalData, InstrumentData, ExecutionTxs, Strategy, Risk>
    Processor<&EngineEvent<InstrumentData::MarketEventKind>>
    for Engine<Clock, EngineState<GlobalData, InstrumentData>, ExecutionTxs, Strategy, Risk>
where
    Clock: EngineClock + for<'a> Processor<&'a EngineEvent<InstrumentData::MarketEventKind>>,
    InstrumentData: InstrumentDataState,
    GlobalData: for<'a> Processor<&'a AccountEvent>
        + for<'a> Processor<&'a MarketEvent<InstrumentIndex, InstrumentData::MarketEventKind>>,
    ExecutionTxs: ExecutionTxMap<ExchangeIndex, InstrumentIndex>,
    Strategy: OnTradingDisabled<Clock, EngineState<GlobalData, InstrumentData>, ExecutionTxs, Risk>
        + OnDisconnectStrategy<Clock, EngineState<GlobalData, InstrumentData>, ExecutionTxs, Risk>
        + AlgoStrategy<State = EngineState<GlobalData, InstrumentData>>
        + ClosePositionsStrategy<State = EngineState<GlobalData, InstrumentData>>,
    Risk: RiskManager<State = EngineState<GlobalData, InstrumentData>>,
{
    type Output = EngineOutput<Strategy::OnTradingDisabled, Strategy::OnDisconnect>;

    fn process(&mut self, event: &EngineEvent<InstrumentData::MarketEventKind>) -> Self::Output {
        // Todo: consider making EngineMeta<Clock> to reduce field count and impl Processor
        self.clock.process(&event);
        self.meta.sequence.increment();

        let event_output = match event {
            EngineEvent::Shutdown(_) => return EngineOutput::Shutdown,
            EngineEvent::Command(command) => {
                let output = self.action(command);
                EngineOutput::Commanded(output)
            }
            EngineEvent::TradingStateUpdate(trading_state) => {
                if let Some(disabled) = self.update_from_trading_state_update(*trading_state) {
                    EngineOutput::OnTradingDisabled(disabled)
                } else {
                    EngineOutput::None
                }
            }
            EngineEvent::Account(account) => match self.update_from_account_stream(account) {
                UpdateFromAccountOutput::None => EngineOutput::None,
                UpdateFromAccountOutput::OnDisconnect(disconnect) => {
                    EngineOutput::AccountDisconnect(disconnect)
                }
                UpdateFromAccountOutput::PositionExit(position) => {
                    EngineOutput::PositionExit(position)
                }
            },
            EngineEvent::Market(market) => match self.update_from_market_stream(market) {
                UpdateFromMarketOutput::None => EngineOutput::None,
                UpdateFromMarketOutput::OnDisconnect(disconnect) => {
                    EngineOutput::MarketDisconnect(disconnect)
                }
            },
        };

        // If trading is enabled, try to generate algo orders
        // If algo orders are generated, return those instead of the event output
        if let TradingState::Enabled = self.state.trading {
            let algo_output = self.generate_algo_orders();
            if !algo_output.is_empty() {
                return EngineOutput::AlgoOrders(algo_output);
            }
        }

        event_output
    }
}

impl<Clock, GlobalData, InstrumentData, ExecutionTxs, Strategy, Risk> SyncShutdown
    for Engine<Clock, EngineState<GlobalData, InstrumentData>, ExecutionTxs, Strategy, Risk>
where
    ExecutionTxs: ExecutionTxMap,
{
    type Result = ();

    fn shutdown(&mut self) -> Self::Result {
        self.execution_txs.iter().for_each(|execution_tx| {
            let _send_result = execution_tx.send(ExecutionRequest::Shutdown);
        });
    }
}

impl<Clock, GlobalData, InstrumentData, ExecutionTxs, Strategy, Risk>
    Engine<Clock, EngineState<GlobalData, InstrumentData>, ExecutionTxs, Strategy, Risk>
{
    /// Action an `Engine` [`Command`], producing an [`ActionOutput`] of work done.
    pub fn action(&mut self, command: &Command) -> ActionOutput
    where
        InstrumentData: InFlightRequestRecorder,
        ExecutionTxs: ExecutionTxMap,
        Strategy: ClosePositionsStrategy<State = EngineState<GlobalData, InstrumentData>>,
        Risk: RiskManager,
    {
        match &command {
            Command::SendCancelRequests(requests) => {
                info!(
                    ?requests,
                    "Engine actioning user Command::SendCancelRequests"
                );
                let output = self.send_requests(requests.clone());
                self.state.record_in_flight_cancels(&output.sent);
                ActionOutput::CancelOrders(output)
            }
            Command::SendOpenRequests(requests) => {
                info!(?requests, "Engine actioning user Command::SendOpenRequests");
                let output = self.send_requests(requests.clone());
                self.state.record_in_flight_opens(&output.sent);
                ActionOutput::OpenOrders(output)
            }
            Command::ClosePositions(filter) => {
                info!(?filter, "Engine actioning user Command::ClosePositions");
                ActionOutput::ClosePositions(self.close_positions(filter))
            }
            Command::CancelOrders(filter) => {
                info!(?filter, "Engine actioning user Command::CancelOrders");
                ActionOutput::CancelOrders(self.cancel_orders(filter))
            }
        }
    }

    /// Update the `Engine` [`TradingState`].
    ///
    /// If the `TradingState` transitions to `TradingState::Disabled`, the `Engine` will call
    /// the configured [`OnTradingDisabled`] strategy logic.
    pub fn update_from_trading_state_update(
        &mut self,
        update: TradingState,
    ) -> Option<Strategy::OnTradingDisabled>
    where
        Strategy:
            OnTradingDisabled<Clock, EngineState<GlobalData, InstrumentData>, ExecutionTxs, Risk>,
    {
        self.state
            .trading
            .update(update)
            .transitioned_to_disabled()
            .then(|| Strategy::on_trading_disabled(self))
    }

    /// Update the [`Engine`] from an [`AccountStreamEvent`].
    ///
    /// If the input `AccountStreamEvent` indicates the exchange execution link has disconnected,
    /// the `Engine` will call the configured [`OnDisconnectStrategy`] strategy logic.
    pub fn update_from_account_stream(
        &mut self,
        event: &AccountStreamEvent,
    ) -> UpdateFromAccountOutput<Strategy::OnDisconnect>
    where
        InstrumentData: for<'a> Processor<&'a AccountEvent>,
        GlobalData: for<'a> Processor<&'a AccountEvent>,
        Strategy: OnDisconnectStrategy<Clock, EngineState<GlobalData, InstrumentData>, ExecutionTxs, Risk>,
    {
        match event {
            AccountStreamEvent::Reconnecting(exchange) => {
                self.state
                    .connectivity
                    .update_from_account_reconnecting(exchange);

                UpdateFromAccountOutput::OnDisconnect(Strategy::on_disconnect(self, *exchange))
            }
            AccountStreamEvent::Item(event) => self
                .state
                .update_from_account(event)
                .map(UpdateFromAccountOutput::PositionExit)
                .unwrap_or(UpdateFromAccountOutput::None),
        }
    }

    /// Update the [`Engine`] from a [`MarketStreamEvent`].
    ///
    /// If the input `MarketStreamEvent` indicates the exchange market data link has disconnected,
    /// the `Engine` will call the configured [`OnDisconnectStrategy`] strategy logic.
    pub fn update_from_market_stream(
        &mut self,
        event: &MarketStreamEvent<InstrumentIndex, InstrumentData::MarketEventKind>,
    ) -> UpdateFromMarketOutput<Strategy::OnDisconnect>
    where
        InstrumentData: InstrumentDataState,
        GlobalData:
            for<'a> Processor<&'a MarketEvent<InstrumentIndex, InstrumentData::MarketEventKind>>,
        Strategy: OnDisconnectStrategy<Clock, EngineState<GlobalData, InstrumentData>, ExecutionTxs, Risk>,
    {
        match event {
            MarketStreamEvent::Reconnecting(exchange) => {
                self.state
                    .connectivity
                    .update_from_market_reconnecting(exchange);

                UpdateFromMarketOutput::OnDisconnect(Strategy::on_disconnect(self, *exchange))
            }
            MarketStreamEvent::Item(event) => {
                self.state.update_from_market(event);
                UpdateFromMarketOutput::None
            }
        }
    }

    /// Returns a [`TradingSummaryGenerator`] for the current trading session.
    pub fn trading_summary_generator(&self, risk_free_return: Decimal) -> TradingSummaryGenerator
    where
        Clock: EngineClock,
    {
        TradingSummaryGenerator::init(
            risk_free_return,
            self.meta.time_start,
            self.time(),
            &self.state.instruments,
            &self.state.assets,
        )
    }
}

impl<Clock, State, ExecutionTxs, Strategy, Risk> Engine<Clock, State, ExecutionTxs, Strategy, Risk>
where
    Clock: EngineClock,
{
    /// Construct a new `Engine`.
    ///
    /// An initial [`EngineMeta`] is constructed form the provided `clock` and `Sequence(0)`.
    pub fn new(
        clock: Clock,
        state: State,
        execution_txs: ExecutionTxs,
        strategy: Strategy,
        risk: Risk,
    ) -> Self {
        Self {
            meta: EngineMeta {
                time_start: clock.time(),
                sequence: Sequence(0),
            },
            clock,
            state,
            execution_txs,
            strategy,
            risk,
        }
    }

    /// Return `Engine` clock time.
    pub fn time(&self) -> DateTime<Utc> {
        self.clock.time()
    }

    /// Reset the internal `EngineMeta` to the `clock` time and `Sequence(0)`.
    pub fn reset_metadata(&mut self) {
        self.meta.time_start = self.clock.time();
        self.meta.sequence = Sequence(0);
    }
}

/// Output produced by [`Engine`] operations, used to construct an `Engine` [`EngineAudit`].
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum EngineOutput<
    OnTradingDisabled,
    OnDisconnect,
    ExchangeKey = ExchangeIndex,
    InstrumentKey = InstrumentIndex,
> {
    None,
    Shutdown,
    Commanded(ActionOutput<ExchangeKey, InstrumentKey>),
    OnTradingDisabled(OnTradingDisabled),
    AccountDisconnect(OnDisconnect),
    PositionExit(PositionExited<QuoteAsset, InstrumentKey>),
    MarketDisconnect(OnDisconnect),
    AlgoOrders(GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>),
}

/// Output produced by the [`Engine`] updating from an [`TradingState`], used to construct
/// an `Engine` [`EngineAudit`].
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum UpdateTradingStateOutput<OnTradingDisabled> {
    None,
    OnTradingDisabled(OnTradingDisabled),
}

/// Output produced by the [`Engine`] updating from an [`AccountStreamEvent`], used to construct
/// an `Engine` [`EngineAudit`].
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum UpdateFromAccountOutput<OnDisconnect, InstrumentKey = InstrumentIndex> {
    None,
    OnDisconnect(OnDisconnect),
    PositionExit(PositionExited<QuoteAsset, InstrumentKey>),
}

/// Output produced by the [`Engine`] updating from an [`MarketStreamEvent`], used to construct
/// an `Engine` [`EngineAudit`].
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum UpdateFromMarketOutput<OnDisconnect> {
    None,
    OnDisconnect(OnDisconnect),
}

impl<OnTradingDisabled, OnDisconnect, ExchangeKey, InstrumentKey>
    From<ActionOutput<ExchangeKey, InstrumentKey>>
    for EngineOutput<OnTradingDisabled, OnDisconnect, ExchangeKey, InstrumentKey>
{
    fn from(value: ActionOutput<ExchangeKey, InstrumentKey>) -> Self {
        Self::Commanded(value)
    }
}

impl<OnTradingDisabled, OnDisconnect, ExchangeKey, InstrumentKey>
    From<PositionExited<QuoteAsset, InstrumentKey>>
    for EngineOutput<OnTradingDisabled, OnDisconnect, ExchangeKey, InstrumentKey>
{
    fn from(value: PositionExited<QuoteAsset, InstrumentKey>) -> Self {
        Self::PositionExit(value)
    }
}

impl<OnTradingDisabled, OnDisconnect, ExchangeKey, InstrumentKey>
    From<GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>>
    for EngineOutput<OnTradingDisabled, OnDisconnect, ExchangeKey, InstrumentKey>
{
    fn from(value: GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>) -> Self {
        Self::AlgoOrders(value)
    }
}

impl<OnTradingDisabled, OnDisconnect, ExchangeKey, InstrumentKey> Terminal
    for EngineOutput<OnTradingDisabled, OnDisconnect, ExchangeKey, InstrumentKey>
{
    fn is_terminal(&self) -> bool {
        matches!(self, EngineOutput::Shutdown)
    }
}
