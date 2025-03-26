use barter::{
    EngineEvent,
    engine::{
        Engine, Processor,
        audit::EngineAudit,
        clock::{EngineClock, LiveClock},
        command::Command,
        run::sync_run_with_audit,
        state::{
            EngineState,
            instrument::{
                data::{DefaultInstrumentMarketData, InstrumentDataState},
                filter::InstrumentFilter,
            },
            position::PositionManager,
            trading::TradingState,
        },
    },
    execution::builder::ExecutionBuilder,
    logging::init_logging,
    risk::{DefaultRiskManager, DefaultRiskManagerState},
    statistic::{summary::instrument::TearSheetGenerator, time::Daily},
    strategy::{
        DefaultStrategy, DefaultStrategyState,
        algo::AlgoStrategy,
        close_positions::{ClosePositionsStrategy, build_ioc_market_order_to_close_position},
        on_disconnect::OnDisconnectStrategy,
        on_trading_disabled::OnTradingDisabled,
    },
};
use barter_data::{
    event::{DataKind, MarketEvent},
    streams::{
        builder::dynamic::indexed::init_indexed_multi_exchange_market_stream,
        reconnect::stream::ReconnectingStream,
    },
    subscription::SubKind,
};
use barter_execution::{
    AccountEvent, AccountEventKind,
    balance::Balance,
    client::mock::MockExecutionConfig,
    order::{
        id::{ClientOrderId, StrategyId},
        request::{OrderRequestCancel, OrderRequestOpen},
    },
};
use barter_instrument::{
    Underlying,
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::{
        Instrument, InstrumentIndex,
        spec::{
            InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity,
            OrderQuantityUnits,
        },
    },
};
use barter_integration::channel::{ChannelTxDroppable, Tx, mpsc_unbounded};
use chrono::{DateTime, Utc};
use fnv::FnvHashMap;
use futures::StreamExt;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tracing::debug;

const EXCHANGE: ExchangeId = ExchangeId::BinanceSpot;
const RISK_FREE_RETURN: Decimal = dec!(0.05);
const MOCK_EXCHANGE_ROUND_TRIP_LATENCY_MS: u64 = 100;
const MOCK_EXCHANGE_FEES_PERCENT: Decimal = dec!(0.05);
const STARTING_BALANCE_USDT: Balance = Balance {
    total: dec!(10_000.0),
    free: dec!(10_000.0),
};
const STARTING_BALANCE_BTC: Balance = Balance {
    total: dec!(0.1),
    free: dec!(0.1),
};
const STARTING_BALANCE_ETH: Balance = Balance {
    total: dec!(1.0),
    free: dec!(1.0),
};
const STARTING_BALANCE_SOL: Balance = Balance {
    total: dec!(10.0),
    free: dec!(10.0),
};

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

#[derive(Debug, Clone)]
struct StrategyCustomInstrumentData {
    tear: TearSheetGenerator,
    position: PositionManager,
}

impl AlgoStrategy for MultiStrategy {
    type State = EngineState<
        MultiStrategyCustomInstrumentData,
        DefaultStrategyState,
        DefaultRiskManagerState,
    >;

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
    type State = EngineState<
        MultiStrategyCustomInstrumentData,
        DefaultStrategyState,
        DefaultRiskManagerState,
    >;

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
    type State = EngineState<
        MultiStrategyCustomInstrumentData,
        DefaultStrategyState,
        DefaultRiskManagerState,
    >;

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
    type State = EngineState<
        MultiStrategyCustomInstrumentData,
        DefaultStrategyState,
        DefaultRiskManagerState,
    >;

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

    // Initialise Channels
    let (feed_tx, mut feed_rx) = mpsc_unbounded();
    let (audit_tx, audit_rx) = mpsc_unbounded();

    // Construct IndexedInstruments
    let instruments = indexed_instruments();

    // Initialise MarketData Stream & forward to Engine feed
    let market_stream = init_indexed_multi_exchange_market_stream(
        &instruments,
        &[SubKind::PublicTrades, SubKind::OrderBooksL1],
    )
    .await?;
    tokio::spawn(market_stream.forward_to(feed_tx.clone()));

    // Construct Engine clock
    let clock = LiveClock;
    let time_engine_start = clock.time();

    // Construct EngineState from IndexedInstruments & hard-coded exchange asset Balances
    let mut state = EngineState::<
        MultiStrategyCustomInstrumentData,
        DefaultStrategyState,
        DefaultRiskManagerState,
    >::builder(&instruments)
    .time_engine_start(time_engine_start)
    .trading_state(TradingState::Enabled)
    .balances([
        (EXCHANGE, "usdt", STARTING_BALANCE_USDT),
        (EXCHANGE, "btc", STARTING_BALANCE_BTC),
        (EXCHANGE, "eth", STARTING_BALANCE_ETH),
        (EXCHANGE, "sol", STARTING_BALANCE_SOL),
    ])
    .build();

    // Update MultiStrategyCustomInstrumentData tear sheets to correct start time - initially
    // each TearSheetGenerator was initialised with the default DateTime<Utc>::MIN_UTC.
    let _ = state
        .instruments
        .instrument_datas_mut(&InstrumentFilter::None)
        .map(|state| {
            state.strategy_a.tear.time_engine_start = time_engine_start;
            state.strategy_a.tear.time_engine_now = time_engine_start;
            state.strategy_b.tear.time_engine_start = time_engine_start;
            state.strategy_b.tear.time_engine_now = time_engine_start;
        });

    // Generate initial AccountSnapshot from EngineState for BinanceSpot MockExchange
    let mut initial_account = FnvHashMap::from(&state);
    assert_eq!(initial_account.len(), 1);

    // Initialise ExecutionManager & forward Account Streams to Engine feed
    let (execution_txs, account_stream) = ExecutionBuilder::new(&instruments)
        .add_mock(MockExecutionConfig::new(
            EXCHANGE,
            initial_account.remove(&EXCHANGE).unwrap(),
            MOCK_EXCHANGE_ROUND_TRIP_LATENCY_MS,
            MOCK_EXCHANGE_FEES_PERCENT,
        ))?
        .init()
        .await?;
    tokio::spawn(account_stream.forward_to(feed_tx.clone()));

    // Construct Engine with our CustomRiskManager
    let mut engine = Engine::new(
        clock,
        state,
        execution_txs,
        DefaultStrategy::default(),
        DefaultRiskManager::default(),
    );

    // Run synchronous Engine on blocking task
    let engine_task = tokio::task::spawn_blocking(move || {
        let shutdown_audit = sync_run_with_audit(
            &mut feed_rx,
            &mut engine,
            &mut ChannelTxDroppable::new(audit_tx),
        );
        (engine, shutdown_audit)
    });

    // Run asynchronous AuditStream consumer to monitor risk decisions
    let audit_task = tokio::spawn(async move {
        let mut audit_stream = audit_rx.into_stream();
        while let Some(audit) = audit_stream.next().await {
            debug!(?audit, "AuditStream consumed AuditTick");
            if let EngineAudit::Shutdown(_) = audit.event {
                break;
            }
        }
        audit_stream
    });

    // Let the example run for 5 seconds..., then:
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    // 1. Disable Strategy order generation
    feed_tx.send(TradingState::Disabled)?;
    // 2. Cancel all open orders
    feed_tx.send(Command::CancelOrders(InstrumentFilter::None))?;
    // 3. Close current positions
    feed_tx.send(Command::ClosePositions(InstrumentFilter::None))?;
    // 4. Stop Engine run loop
    feed_tx.send(EngineEvent::Shutdown)?;

    // Await Engine & AuditStream task graceful shutdown
    let (engine, _shutdown_audit) = engine_task.await?;
    let _audit_stream = audit_task.await?;

    // Generate TradingSummary<Daily>
    let trading_summary = engine
        .trading_summary_generator(RISK_FREE_RETURN)
        .generate(Daily);

    // Print TradingSummary<Daily> to terminal
    trading_summary.print_summary();

    Ok(())
}

fn indexed_instruments() -> IndexedInstruments {
    IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            EXCHANGE,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(
                    OrderQuantityUnits::Quote,
                    dec!(0.00001),
                    dec!(0.00001),
                ),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .add_instrument(Instrument::spot(
            EXCHANGE,
            "binance_spot_eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.0001), dec!(0.0001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .add_instrument(Instrument::spot(
            EXCHANGE,
            "binance_spot_sol_usdt",
            "SOLUSDT",
            Underlying::new("sol", "usdt"),
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.001), dec!(0.001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .build()
}
