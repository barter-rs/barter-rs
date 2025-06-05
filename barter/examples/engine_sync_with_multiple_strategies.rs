use barter::{
    EngineEvent,
    engine::{
        Engine, Processor,
        clock::LiveClock,
        state::{
            EngineState,
            global::DefaultGlobalData,
            instrument::{
                data::{DefaultInstrumentMarketData, InstrumentDataState},
                filter::InstrumentFilter,
            },
            order::in_flight_recorder::InFlightRequestRecorder,
            position::PositionManager,
            trading::TradingState,
        },
    },
    logging::init_logging,
    risk::DefaultRiskManager,
    statistic::{summary::instrument::TearSheetGenerator, time::Daily},
    strategy::{
        DefaultStrategy,
        algo::AlgoStrategy,
        close_positions::{ClosePositionsStrategy, build_ioc_market_order_to_close_position},
        on_disconnect::OnDisconnectStrategy,
        on_trading_disabled::OnTradingDisabled,
    },
    system::{
        builder::{AuditMode, EngineFeedMode, SystemArgs, SystemBuilder},
        config::SystemConfig,
    },
};
use barter_data::{
    event::{DataKind, MarketEvent},
    streams::builder::dynamic::indexed::init_indexed_multi_exchange_market_stream,
    subscription::SubKind,
};
use barter_execution::{
    AccountEvent, AccountEventKind,
    order::{
        id::{ClientOrderId, StrategyId},
        request::{OrderRequestCancel, OrderRequestOpen},
    },
};
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::InstrumentIndex,
};
use barter_integration::Terminal;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use std::{fs::File, io::BufReader, time::Duration};
use tracing::debug;

const FILE_PATH_SYSTEM_CONFIG: &str = "barter/examples/config/system_config.json";
const RISK_FREE_RETURN: Decimal = dec!(0.05);

struct MultiStrategy {
    strategy_a: StrategyA,
    strategy_b: StrategyB,
}

#[derive(Debug, Clone, Default)]
struct MultiStrategyCustomInstrumentData {
    market_data: DefaultInstrumentMarketData,
    strategy_a: StrategyCustomInstrumentData,
    strategy_b: StrategyCustomInstrumentData,
}

impl MultiStrategyCustomInstrumentData {
    pub fn init(time_engine_start: DateTime<Utc>) -> Self {
        Self {
            market_data: DefaultInstrumentMarketData::default(),
            strategy_a: StrategyCustomInstrumentData::init(time_engine_start),
            strategy_b: StrategyCustomInstrumentData::init(time_engine_start),
        }
    }
}

#[derive(Debug, Clone)]
struct StrategyCustomInstrumentData {
    tear: TearSheetGenerator,
    position: PositionManager,
}

impl StrategyCustomInstrumentData {
    pub fn init(time_engine_start: DateTime<Utc>) -> Self {
        Self {
            tear: TearSheetGenerator::init(time_engine_start),
            position: PositionManager::default(),
        }
    }
}

impl AlgoStrategy for MultiStrategy {
    type State = EngineState<DefaultGlobalData, MultiStrategyCustomInstrumentData>;

    fn generate_algo_orders(
        &self,
        state: &Self::State,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>>,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>>,
    ) {
        let (cancels_a, opens_a) = self.strategy_a.generate_algo_orders(state);
        let (cancels_b, opens_b) = self.strategy_b.generate_algo_orders(state);

        let cancels_all = cancels_a.into_iter().chain(cancels_b);
        let opens_all = opens_a.into_iter().chain(opens_b);

        (cancels_all, opens_all)
    }
}

impl ClosePositionsStrategy for MultiStrategy {
    type State = EngineState<DefaultGlobalData, MultiStrategyCustomInstrumentData>;

    fn close_positions_requests<'a>(
        &'a self,
        state: &'a Self::State,
        filter: &'a InstrumentFilter,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel> + 'a,
        impl IntoIterator<Item = OrderRequestOpen> + 'a,
    )
    where
        ExchangeIndex: 'a,
        AssetIndex: 'a,
        InstrumentIndex: 'a,
    {
        // Generate a MARKET order for each Strategy's open Position
        let open_requests =
            state
                .instruments
                .instruments(filter)
                .flat_map(move |state| {
                    // Only generate orders if we have a market price
                    let Some(price) = state.data.price() else {
                        return itertools::Either::Left(std::iter::empty());
                    };

                    // Generate a MARKET order to close StrategyA position
                    let close_position_a_request = state
                        .data
                        .strategy_a
                        .position
                        .current
                        .as_ref()
                        .map(|position_a| {
                            build_ioc_market_order_to_close_position(
                                state.instrument.exchange,
                                position_a,
                                StrategyA::ID,
                                price,
                                || ClientOrderId::random(),
                            )
                        });

                    // Generate a MARKET order to close StrategyB position
                    let close_position_b_request = state
                        .data
                        .strategy_b
                        .position
                        .current
                        .as_ref()
                        .map(|position_b| {
                            build_ioc_market_order_to_close_position(
                                state.instrument.exchange,
                                position_b,
                                StrategyB::ID,
                                price,
                                || ClientOrderId::random(),
                            )
                        });

                    itertools::Either::Right(
                        close_position_a_request
                            .into_iter()
                            .chain(close_position_b_request),
                    )
                });

        (std::iter::empty(), open_requests)
    }
}

impl<Clock, State, ExecutionTxs, Risk> OnDisconnectStrategy<Clock, State, ExecutionTxs, Risk>
    for MultiStrategy
{
    type OnDisconnect = ();

    fn on_disconnect(
        _: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
        _: ExchangeId,
    ) -> Self::OnDisconnect {
    }
}

impl<Clock, State, ExecutionTxs, Risk> OnTradingDisabled<Clock, State, ExecutionTxs, Risk>
    for MultiStrategy
{
    type OnTradingDisabled = ();

    fn on_trading_disabled(
        _: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
    ) -> Self::OnTradingDisabled {
    }
}

struct StrategyA;

impl StrategyA {
    const ID: StrategyId = StrategyId(SmolStr::new_static("strategy_a"));
}

impl AlgoStrategy for StrategyA {
    type State = EngineState<DefaultGlobalData, MultiStrategyCustomInstrumentData>;

    fn generate_algo_orders(
        &self,
        _: &Self::State,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>>,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }
}

struct StrategyB;

impl StrategyB {
    const ID: StrategyId = StrategyId(SmolStr::new_static("strategy_b"));
}

impl AlgoStrategy for StrategyB {
    type State = EngineState<DefaultGlobalData, MultiStrategyCustomInstrumentData>;

    fn generate_algo_orders(
        &self,
        _: &Self::State,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>>,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }
}

impl InstrumentDataState for MultiStrategyCustomInstrumentData {
    type MarketEventKind = DataKind;

    fn price(&self) -> Option<Decimal> {
        self.market_data.price()
    }
}

impl<InstrumentKey> Processor<&MarketEvent<InstrumentKey, DataKind>>
    for MultiStrategyCustomInstrumentData
{
    type Audit = ();

    fn process(&mut self, event: &MarketEvent<InstrumentKey, DataKind>) -> Self::Audit {
        self.market_data.process(event)
    }
}

impl Processor<&AccountEvent> for MultiStrategyCustomInstrumentData {
    type Audit = ();

    fn process(&mut self, event: &AccountEvent) -> Self::Audit {
        let AccountEventKind::Trade(trade) = &event.kind else {
            return;
        };

        if trade.strategy == StrategyA::ID {
            self.strategy_a
                .position
                .update_from_trade(trade)
                .inspect(|closed| self.strategy_a.tear.update_from_position(closed));
        }

        if trade.strategy == StrategyB::ID {
            self.strategy_b
                .position
                .update_from_trade(trade)
                .inspect(|closed| self.strategy_b.tear.update_from_position(closed));
        }
    }
}

impl InFlightRequestRecorder for MultiStrategyCustomInstrumentData {
    fn record_in_flight_cancel(&mut self, _: &OrderRequestCancel<ExchangeIndex, InstrumentIndex>) {}

    fn record_in_flight_open(&mut self, _: &OrderRequestOpen<ExchangeIndex, InstrumentIndex>) {}
}

impl Default for StrategyCustomInstrumentData {
    fn default() -> Self {
        Self {
            tear: TearSheetGenerator::init(DateTime::<Utc>::MIN_UTC),
            position: Default::default(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise Tracing
    init_logging();

    // Load SystemConfig
    let SystemConfig {
        instruments,
        executions,
    } = load_config()?;

    // Construct IndexedInstruments
    let instruments = IndexedInstruments::new(instruments);

    // Initialise MarketData Stream
    let market_stream = init_indexed_multi_exchange_market_stream(
        &instruments,
        &[SubKind::PublicTrades, SubKind::OrderBooksL1],
    )
    .await?;

    // Construct System Args
    let args = SystemArgs::new(
        &instruments,
        executions,
        LiveClock,
        DefaultStrategy::default(),
        DefaultRiskManager::default(),
        market_stream,
        DefaultGlobalData::default(),
        |_| MultiStrategyCustomInstrumentData::init(Utc::now()),
    );

    // Build & run System:
    // See SystemBuilder for all configuration options
    let mut system = SystemBuilder::new(args)
        // Engine feed in Sync mode (Iterator input)
        .engine_feed_mode(EngineFeedMode::Iterator)
        // Audit feed is enabled (Engine sends audits)
        .audit_mode(AuditMode::Enabled)
        // Engine starts with TradingState::Disabled
        .trading_state(TradingState::Disabled)
        // Build System, but don't start spawning tasks yet
        .build::<EngineEvent, _>()?
        // Init System, spawning component tasks on the current runtime
        .init_with_runtime(tokio::runtime::Handle::current())
        .await?;

    // Take ownership of the Engine audit snapshot with updates
    let audit = system.audit.take().unwrap();

    // Run dummy asynchronous AuditStream consumer
    // Note: you probably want to use this Stream to replicate EngineState, or persist events, etc.
    //  --> eg/ see examples/engine_sync_with_audit_replica_engine_state
    let audit_task = tokio::spawn(async move {
        let mut audit_stream = audit.updates.into_stream();
        while let Some(audit) = audit_stream.next().await {
            debug!(?audit, "AuditStream consumed AuditTick");
            if audit.event.is_terminal() {
                break;
            }
        }
        audit_stream
    });

    // Enable trading
    system.trading_state(TradingState::Enabled);

    // Let the example run for 5 seconds...
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Before shutting down, CancelOrders and then ClosePositions
    system.cancel_orders(InstrumentFilter::None);
    system.close_positions(InstrumentFilter::None);

    // Shutdown
    let (engine, _shutdown_audit) = system.shutdown().await?;
    let _audit_stream = audit_task.await?;

    // Generate TradingSummary<Daily>
    let trading_summary = engine
        .trading_summary_generator(RISK_FREE_RETURN)
        .generate(Daily);

    // Print TradingSummary<Daily> to terminal (could save in a file, send somewhere, etc.)
    trading_summary.print_summary();

    Ok(())
}

fn load_config() -> Result<SystemConfig, Box<dyn std::error::Error>> {
    let file = File::open(FILE_PATH_SYSTEM_CONFIG)?;
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader)?;
    Ok(config)
}
