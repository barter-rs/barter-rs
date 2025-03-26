use crate::{
    EngineEvent,
    engine::{
        Engine, EngineOutput, Processor,
        clock::EngineClock,
        execution_tx::MultiExchangeTxMap,
        run, run_async,
        state::{
            EngineState, builder::EngineStateBuilder, instrument::data::InstrumentDataState,
            trading::TradingState,
        },
    },
    error::BarterError,
    execution::builder::ExecutionBuilder,
    risk::RiskManager,
    strategy::{
        algo::AlgoStrategy, close_positions::ClosePositionsStrategy,
        on_disconnect::OnDisconnectStrategy, on_trading_disabled::OnTradingDisabled,
    },
    system::{AuditMode, EngineFeedMode, ExecutionConfig, System, SystemHandles},
};
use barter_data::{
    event::MarketEvent,
    streams::{consumer::MarketStreamEvent, reconnect::stream::ReconnectingStream},
};
use barter_execution::AccountEvent;
use barter_instrument::{index::IndexedInstruments, instrument::InstrumentIndex};
use barter_integration::channel::{ChannelTxDroppable, mpsc_unbounded};
use futures::Stream;
use std::{fmt::Debug, sync::Arc};

#[derive(Debug)]
pub struct SystemConfig<Clock, Strategy, Risk, MarketStream> {
    pub instruments: Arc<IndexedInstruments>,
    pub executions: Vec<ExecutionConfig>,
    pub clock: Clock,
    pub strategy: Strategy,
    pub risk: Risk,
    pub market_stream: MarketStream,
}

#[derive(Debug)]
pub struct SystemBuilder<Clock, Strategy, Risk, MarketStream> {
    config: SystemConfig<Clock, Strategy, Risk, MarketStream>,
    runtime: Option<tokio::runtime::Handle>,
    engine_feed_mode: Option<EngineFeedMode>,
    audit_mode: Option<AuditMode>,
    trading_state: Option<TradingState>,
}

impl<Clock, Strategy, Risk, MarketStream> SystemBuilder<Clock, Strategy, Risk, MarketStream> {
    pub fn new(config: SystemConfig<Clock, Strategy, Risk, MarketStream>) -> Self {
        Self {
            config,
            runtime: None,
            engine_feed_mode: None,
            audit_mode: None,
            trading_state: None,
        }
    }

    pub fn runtime(self, value: tokio::runtime::Handle) -> Self {
        Self {
            runtime: Some(value),
            ..self
        }
    }

    pub fn engine_feed_mode(self, value: EngineFeedMode) -> Self {
        Self {
            engine_feed_mode: Some(value),
            ..self
        }
    }

    pub fn audit_mode(self, value: AuditMode) -> Self {
        Self {
            audit_mode: Some(value),
            ..self
        }
    }

    pub fn trading_state(self, value: TradingState) -> Self {
        Self {
            trading_state: Some(value),
            ..self
        }
    }

    pub async fn build<InstrumentData, StrategyState, RiskState>(
        self,
    ) -> Result<
        System<
            Engine<
                Clock,
                EngineState<InstrumentData, StrategyState, RiskState>,
                MultiExchangeTxMap,
                Strategy,
                Risk,
            >,
            EngineEvent<InstrumentData::MarketEventKind>,
            EngineOutput<Strategy::OnTradingDisabled, Strategy::OnDisconnect>,
            EngineState<InstrumentData, StrategyState, RiskState>,
        >,
        BarterError,
    >
    where
        Clock: EngineClock
            + for<'a> Processor<&'a EngineEvent<InstrumentData::MarketEventKind>>
            + Send
            + 'static,
        Strategy: AlgoStrategy<State = EngineState<InstrumentData, StrategyState, RiskState>>
            + ClosePositionsStrategy<State = EngineState<InstrumentData, StrategyState, RiskState>>
            + OnTradingDisabled<
                Clock,
                EngineState<InstrumentData, StrategyState, RiskState>,
                MultiExchangeTxMap,
                Risk,
            > + OnDisconnectStrategy<
                Clock,
                EngineState<InstrumentData, StrategyState, RiskState>,
                MultiExchangeTxMap,
                Risk,
            > + Send
            + 'static,
        <Strategy as OnTradingDisabled<
            Clock,
            EngineState<InstrumentData, StrategyState, RiskState>,
            MultiExchangeTxMap,
            Risk,
        >>::OnTradingDisabled: Debug + Clone + Send,
        <Strategy as OnDisconnectStrategy<
            Clock,
            EngineState<InstrumentData, StrategyState, RiskState>,
            MultiExchangeTxMap,
            Risk,
        >>::OnDisconnect: Debug + Clone + Send,
        Risk: RiskManager<State = EngineState<InstrumentData, StrategyState, RiskState>>
            + Send
            + 'static,
        MarketStream: Stream<Item = MarketStreamEvent<InstrumentIndex, InstrumentData::MarketEventKind>>
            + Send
            + 'static,
        InstrumentData: InstrumentDataState + Default + Send + 'static,
        StrategyState: for<'a> Processor<&'a AccountEvent>
            + for<'a> Processor<&'a MarketEvent<InstrumentIndex, InstrumentData::MarketEventKind>>
            + Debug
            + Clone
            + Default
            + Send
            + 'static,
        RiskState: for<'a> Processor<&'a AccountEvent>
            + for<'a> Processor<&'a MarketEvent<InstrumentIndex, InstrumentData::MarketEventKind>>
            + Debug
            + Clone
            + Default
            + Send
            + 'static,
    {
        let Self {
            config:
                SystemConfig {
                    instruments,
                    executions,
                    clock,
                    strategy,
                    risk,
                    market_stream,
                },
            runtime,
            engine_feed_mode,
            audit_mode: _,
            trading_state,
        } = self;

        let runtime = runtime.unwrap_or_else(|| tokio::runtime::Handle::current());
        let engine_feed_mode = engine_feed_mode.unwrap_or_default();
        let trading_state = trading_state.unwrap_or_default();

        // Initialise Channels
        let (feed_tx, mut feed_rx) = mpsc_unbounded();
        let (audit_tx, audit_rx) = mpsc_unbounded();

        // Forward MarketStreamEvents to Engine feed
        let market_to_engine = runtime.spawn(market_stream.forward_to(feed_tx.clone()));

        // Build Execution infrastructure
        let execution_builder = executions.into_iter().try_fold(
            ExecutionBuilder::new_with_runtime(runtime.clone(), &instruments),
            |builder, config| match config {
                ExecutionConfig::Mock(mock_config) => builder.add_mock(mock_config),
            },
        )?;

        // Initialise Execution infrastructure
        let (execution_txs, account_stream) = execution_builder.init().await?;

        // Forward AccountStreamEvents to Engine feed
        let account_to_engine = runtime.spawn(account_stream.forward_to(feed_tx.clone()));

        // Build EngineState
        let state = EngineStateBuilder::new(&instruments)
            .time_engine_start(clock.time())
            .trading_state(trading_state)
            .build();

        // Construct Engine
        let mut engine = Engine::new(clock, state, execution_txs, strategy, risk);

        // Todo: audit configuration
        let mut audit_tx = ChannelTxDroppable::new(audit_tx);

        // Run Engine in blocking (Sync) or non-blocking (Async) mode
        let engine = match engine_feed_mode {
            EngineFeedMode::Sync => runtime.spawn_blocking(move || {
                let shutdown_audit = run(&mut feed_rx, &mut engine, &mut audit_tx);
                (engine, shutdown_audit)
            }),
            EngineFeedMode::Async => runtime.spawn(async move {
                let shutdown_audit = run_async(&mut feed_rx, &mut engine, &mut audit_tx).await;
                (engine, shutdown_audit)
            }),
        };

        Ok(System {
            handles: SystemHandles {
                runtime,
                engine,
                market_to_engine,
                account_to_engine,
            },
            feed_tx,
            audit_rx: Some(audit_rx),
        })
    }
}
