use crate::{
    engine::{
        action::{
            cancel_orders::CancelOrders,
            close_positions::ClosePositions,
            generate_algo_orders::{GenerateAlgoOrders, GenerateAlgoOrdersOutput},
            send_requests::SendRequests,
            ActionOutput,
        },
        audit::{context::EngineContext, Audit, AuditTick, Auditor, DefaultAudit},
        command::Command,
        execution_tx::ExecutionTxMap,
        state::{
            instrument::market_data::MarketDataState,
            order::in_flight_recorder::InFlightRequestRecorder, trading::TradingState, EngineState,
        },
    },
    execution::AccountStreamEvent,
    risk::RiskManager,
    statistic::summary::TradingSummaryGenerator,
    strategy::{
        algo::AlgoStrategy, close_positions::ClosePositionsStrategy,
        on_disconnect::OnDisconnectStrategy, on_trading_disabled::OnTradingDisabled,
    },
    EngineEvent, Sequence,
};
use audit::shutdown::ShutdownAudit;
use barter_data::{event::MarketEvent, streams::consumer::MarketStreamEvent};
use barter_execution::AccountEvent;
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};
use barter_integration::channel::{ChannelTxDroppable, Tx};
use chrono::{DateTime, Utc};
use derive_more::From;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::info;

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
    Events::Item: Debug + Clone,
    Engine: Processor<Events::Item> + Auditor<Engine::Output, Context = EngineContext>,
    Engine::Output: From<Engine::Snapshot> + From<ShutdownAudit<Events::Item>>,
    AuditTx: Tx<Item = AuditTick<Engine::Output, EngineContext>>,
    Option<ShutdownAudit<Events::Item>>: for<'a> From<&'a Engine::Output>,
{
    info!("Engine running");

    // Send initial Engine state snapshot
    audit_tx.send(engine.audit(engine.snapshot()));

    // Run Engine process loop until shutdown
    let shutdown_audit = loop {
        let Some(event) = feed.next() else {
            audit_tx.send(engine.audit(ShutdownAudit::FeedEnded));
            break ShutdownAudit::FeedEnded;
        };

        // Process Event & check if Output indicates shutdown is required
        let audit_kind = engine.process(event);
        let shutdown = Option::<ShutdownAudit<Events::Item>>::from(&audit_kind);

        // Send AuditTick to AuditManager
        audit_tx.send(engine.audit(audit_kind));

        if let Some(shutdown) = shutdown {
            break shutdown;
        }
    };

    // Send Shutdown audit
    audit_tx.send(engine.audit(shutdown_audit.clone()));

    info!(?shutdown_audit, "Engine shutting down");
    shutdown_audit
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Engine<State, ExecutionTxs, Strategy, Risk> {
    pub clock: fn() -> DateTime<Utc>,
    pub meta: EngineMeta,
    pub state: State,
    pub execution_txs: ExecutionTxs,
    pub strategy: Strategy,
    pub risk: Risk,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct EngineMeta {
    pub time_start: DateTime<Utc>,
    pub sequence: Sequence,
}

impl<MarketState, StrategyState, RiskState, ExecutionTxs, Strategy, Risk>
    Processor<EngineEvent<MarketState::EventKind>>
    for Engine<EngineState<MarketState, StrategyState, RiskState>, ExecutionTxs, Strategy, Risk>
where
    MarketState: MarketDataState,
    StrategyState: for<'a> Processor<&'a AccountEvent>
        + for<'a> Processor<&'a MarketEvent<InstrumentIndex, MarketState::EventKind>>,
    RiskState: for<'a> Processor<&'a AccountEvent>
        + for<'a> Processor<&'a MarketEvent<InstrumentIndex, MarketState::EventKind>>,
    ExecutionTxs: ExecutionTxMap<ExchangeIndex, InstrumentIndex>,
    Strategy: OnTradingDisabled<EngineState<MarketState, StrategyState, RiskState>, ExecutionTxs, Risk>
        + OnDisconnectStrategy<EngineState<MarketState, StrategyState, RiskState>, ExecutionTxs, Risk>
        + AlgoStrategy<State = EngineState<MarketState, StrategyState, RiskState>>
        + ClosePositionsStrategy<State = EngineState<MarketState, StrategyState, RiskState>>,
    Risk: RiskManager<State = EngineState<MarketState, StrategyState, RiskState>>,
{
    type Output = DefaultAudit<
        MarketState,
        StrategyState,
        RiskState,
        Strategy::OnTradingDisabled,
        Strategy::OnDisconnect,
    >;

    fn process(&mut self, event: EngineEvent<MarketState::EventKind>) -> Self::Output {
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

        if let TradingState::Enabled = self.state.trading {
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

impl<MarketState, StrategyState, RiskState, ExecutionTxs, Strategy, Risk>
    Engine<EngineState<MarketState, StrategyState, RiskState>, ExecutionTxs, Strategy, Risk>
{
    pub fn action(&mut self, command: &Command) -> ActionOutput
    where
        ExecutionTxs: ExecutionTxMap,
        Strategy:
            ClosePositionsStrategy<State = EngineState<MarketState, StrategyState, RiskState>>,
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

    pub fn update_from_trading_state_update(
        &mut self,
        update: TradingState,
    ) -> Option<Strategy::OnTradingDisabled>
    where
        Strategy: OnTradingDisabled<
            EngineState<MarketState, StrategyState, RiskState>,
            ExecutionTxs,
            Risk,
        >,
    {
        self.state
            .trading
            .update(update)
            .transitioned_to_disabled()
            .then(|| Strategy::on_trading_disabled(self))
    }

    pub fn update_from_account_stream(
        &mut self,
        event: &AccountStreamEvent,
    ) -> Option<Strategy::OnDisconnect>
    where
        StrategyState: for<'a> Processor<&'a AccountEvent>,
        RiskState: for<'a> Processor<&'a AccountEvent>,
        Strategy: OnDisconnectStrategy<
            EngineState<MarketState, StrategyState, RiskState>,
            ExecutionTxs,
            Risk,
        >,
    {
        match event {
            AccountStreamEvent::Reconnecting(exchange) => {
                self.state
                    .connectivity
                    .update_from_account_reconnecting(exchange);
                Some(Strategy::on_disconnect(self, *exchange))
            }
            AccountStreamEvent::Item(event) => {
                let _position = self.state.update_from_account(event);
                None
            }
        }
    }

    pub fn update_from_market_stream(
        &mut self,
        event: &MarketStreamEvent<InstrumentIndex, MarketState::EventKind>,
    ) -> Option<Strategy::OnDisconnect>
    where
        MarketState: MarketDataState,
        StrategyState: for<'a> Processor<&'a MarketEvent<InstrumentIndex, MarketState::EventKind>>,
        RiskState: for<'a> Processor<&'a MarketEvent<InstrumentIndex, MarketState::EventKind>>,
        Strategy: OnDisconnectStrategy<
            EngineState<MarketState, StrategyState, RiskState>,
            ExecutionTxs,
            Risk,
        >,
    {
        match event {
            MarketStreamEvent::Reconnecting(exchange) => {
                self.state
                    .connectivity
                    .update_from_market_reconnecting(exchange);
                Some(Strategy::on_disconnect(self, *exchange))
            }
            MarketStreamEvent::Item(event) => {
                self.state.update_from_market(event);
                None
            }
        }
    }

    pub fn trading_summary_generator(&self, risk_free_return: Decimal) -> TradingSummaryGenerator {
        TradingSummaryGenerator::init(
            risk_free_return,
            self.meta.time_start,
            &self.state.instruments,
            &self.state.assets,
        )
    }
}

impl<State, ExecutionTxs, Strategy, Risk> Engine<State, ExecutionTxs, Strategy, Risk> {
    /// Construct a new `Engine`.
    ///
    /// An initial [`EngineMeta`] is constructed form the provided `clock` and `Sequence(0)`.
    pub fn new(
        clock: fn() -> DateTime<Utc>,
        state: State,
        execution_txs: ExecutionTxs,
        strategy: Strategy,
        risk: Risk,
    ) -> Self {
        Self {
            meta: EngineMeta {
                time_start: clock(),
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
        (self.clock)()
    }

    /// Reset the internal `EngineMeta` to the `clock` time and `Sequence(0)`.
    pub fn reset_metadata(&mut self) {
        self.meta.time_start = (self.clock)();
        self.meta.sequence = Sequence(0);
    }
}

impl<Audit, State, ExecutionTx, StrategyT, Risk> Auditor<Audit>
    for Engine<State, ExecutionTx, StrategyT, Risk>
where
    Audit: From<State>,
    State: Clone,
{
    type Context = EngineContext;
    type Snapshot = State;
    type Shutdown<Event> = ShutdownAudit<Event>;

    fn snapshot(&self) -> Self::Snapshot {
        self.state.clone()
    }

    fn audit<Kind>(&mut self, kind: Kind) -> AuditTick<Audit, EngineContext>
    where
        Audit: From<Kind>,
    {
        AuditTick {
            event: Audit::from(kind),
            context: EngineContext {
                sequence: self.meta.sequence.fetch_add(),
                time: (self.clock)(),
            },
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum EngineOutput<
    OnTradingDisabled,
    OnDisconnect,
    ExchangeKey = ExchangeIndex,
    InstrumentKey = InstrumentIndex,
> {
    Commanded(ActionOutput<ExchangeKey, InstrumentKey>),
    OnTradingDisabled(OnTradingDisabled),
    OnDisconnect(OnDisconnect),
    AlgoOrders(GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>),
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
    From<GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>>
    for EngineOutput<OnTradingDisabled, OnDisconnect, ExchangeKey, InstrumentKey>
{
    fn from(value: GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>) -> Self {
        Self::AlgoOrders(value)
    }
}
