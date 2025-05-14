use barter::{
    backtest,
    backtest::{BacktestArgsConstant, BacktestArgsDynamic, market_data::MarketDataInMemory},
    engine::{
        Engine, Processor,
        clock::HistoricalClock,
        execution_tx::MultiExchangeTxMap,
        state::{
            EngineState,
            builder::EngineStateBuilder,
            global::DefaultGlobalData,
            instrument::{
                data::{DefaultInstrumentMarketData, InstrumentDataState},
                filter::InstrumentFilter,
            },
            order::in_flight_recorder::InFlightRequestRecorder,
            trading::TradingState,
        },
    },
    risk::DefaultRiskManager,
    statistic::time::Daily,
    strategy::{
        algo::AlgoStrategy,
        close_positions::{ClosePositionsStrategy, close_open_positions_with_market_orders},
        on_disconnect::OnDisconnectStrategy,
        on_trading_disabled::OnTradingDisabled,
    },
    system::config::{ExecutionConfig, InstrumentConfig, SystemConfig},
};
use barter_data::{
    event::{DataKind, MarketEvent},
    streams::consumer::MarketStreamEvent,
    subscription::trade::PublicTrade,
};
use barter_execution::{
    AccountEvent,
    order::{
        OrderKey, OrderKind, TimeInForce,
        id::{ClientOrderId, StrategyId},
        request::{OrderRequestCancel, OrderRequestOpen, RequestOpen},
    },
};
use barter_instrument::{
    Side,
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::InstrumentIndex,
};
use chrono::{DateTime, Utc};
use criterion::{Criterion, Throughput};
use rust_decimal::{Decimal, prelude::FromPrimitive};
use serde::Deserialize;
use smol_str::SmolStr;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    str::FromStr,
    sync::Arc,
};

criterion::criterion_main!(benchmark_backtest);

// Config containing max balances to enable spamming open order requests
const CONFIG: &str = r#"
{
  "risk_free_return": 0.05,
  "system": {
    "executions": [
      {
        "mocked_exchange": "binance_spot",
        "latency_ms": 100,
        "fees_percent": 0.05,
        "initial_state": {
          "exchange": "binance_spot",
          "balances": [
            {
              "asset": "usdt",
              "balance": {
                "total": 99999999999999,
                "free": 99999999999999
              },
              "time_exchange": "2025-03-24T21:30:00Z"
            },
            {
              "asset": "btc",
              "balance": {
                "total": 99999999999999,
                "free": 99999999999999
              },
              "time_exchange": "2025-03-24T21:30:00Z"
            },
            {
              "asset": "eth",
              "balance": {
                "total": 99999999999999,
                "free": 99999999999999
              },
              "time_exchange": "2025-03-24T21:30:00Z"
            },
            {
              "asset": "sol",
              "balance": {
                "total": 99999999999999,
                "free": 99999999999999
              },
              "time_exchange": "2025-03-24T21:30:00Z"
            }
          ],
          "instruments": [
            {
              "instrument": "BTCUSDT",
              "orders": []
            },
            {
              "instrument": "ETHUSDT",
              "orders": []
            },
            {
              "instrument": "SOLUSDT",
              "orders": []
            }
          ]
        }
      }
    ],
    "instruments": [
      {
        "exchange": "binance_spot",
        "name_exchange": "BTCUSDT",
        "underlying": {
          "base": "btc",
          "quote": "usdt"
        },
        "quote": "underlying_quote",
        "kind": "spot"
      },
      {
        "exchange": "binance_spot",
        "name_exchange": "ETHUSDT",
        "underlying": {
          "base": "eth",
          "quote": "usdt"
        },
        "quote": "underlying_quote",
        "kind": "spot"
      },
      {
        "exchange": "binance_spot",
        "name_exchange": "SOLUSDT",
        "underlying": {
          "base": "sol",
          "quote": "usdt"
        },
        "quote": "underlying_quote",
        "kind": "spot"
      }
    ]
  }
}
"#;

const FILE_PATH_MARKET_DATA_INDEXED: &str =
    "examples/data/binance_spot_trades_l1_btcusdt_ethusdt_solusdt.json";

#[derive(Deserialize)]
pub struct Config {
    pub risk_free_return: Decimal,
    pub system: SystemConfig,
}

fn benchmark_backtest() {
    let Config {
        risk_free_return,
        system: SystemConfig {
            instruments,
            executions,
        },
    } = serde_json::from_str(CONFIG).unwrap();

    let args_constant = args_constant(instruments, executions);
    let args_dynamic = args_dynamic(risk_free_return);

    let mut c = Criterion::default().without_plots();

    bench_backtest(&mut c, Arc::clone(&args_constant), &args_dynamic);
    bench_backtests_concurrent(&mut c, args_constant, args_dynamic);
}

fn bench_backtest(
    c: &mut Criterion,
    args_constant: Arc<
        BacktestArgsConstant<
            MarketDataInMemory<DataKind>,
            Daily,
            EngineState<DefaultGlobalData, LoseMoneyInstrumentData>,
        >,
    >,
    args_dynamic: &BacktestArgsDynamic<
        LoseMoneyStrategy,
        DefaultRiskManager<EngineState<DefaultGlobalData, LoseMoneyInstrumentData>>,
    >,
) {
    let mut group = c.benchmark_group("Backtest");
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(10));
    group.sample_size(50);
    group.throughput(Throughput::Elements(1));

    group.bench_function("Single", |b| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        b.iter_batched(
            || (Arc::clone(&args_constant), args_dynamic.clone()),
            |(constant, dynamic)| {
                rt.block_on(async move { backtest::backtest(constant, dynamic).await.unwrap() })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_backtests_concurrent(
    c: &mut Criterion,
    args_constant: Arc<
        BacktestArgsConstant<
            MarketDataInMemory<DataKind>,
            Daily,
            EngineState<DefaultGlobalData, LoseMoneyInstrumentData>,
        >,
    >,
    args_dynamic: BacktestArgsDynamic<
        LoseMoneyStrategy,
        DefaultRiskManager<EngineState<DefaultGlobalData, LoseMoneyInstrumentData>>,
    >,
) {
    let bench_func = |b: &mut criterion::Bencher, num_concurrent| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        b.iter_batched(
            || {
                let dynamics = (0..num_concurrent)
                    .map(|_| args_dynamic.clone())
                    .collect::<Vec<_>>();

                (Arc::clone(&args_constant), dynamics)
            },
            |(constant, dynamics)| {
                rt.block_on(async move {
                    backtest::run_backtests(constant, dynamics).await.unwrap();
                });
            },
            criterion::BatchSize::SmallInput,
        );
    };

    // 10 concurrent backtests
    let mut group = c.benchmark_group("Backtest Concurrent");
    group.throughput(Throughput::Elements(10));
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(15));
    group.sample_size(50);
    group.bench_function("10", |b| bench_func(b, 10));
    group.finish();

    // 500 concurrent backtests
    let mut group = c.benchmark_group("Backtest Concurrent");
    group.throughput(Throughput::Elements(500));
    group.warm_up_time(std::time::Duration::from_secs(10));
    group.measurement_time(std::time::Duration::from_secs(120));
    group.sample_size(10);
    group.bench_function("500", |b| bench_func(b, 500));
    group.finish();
}

#[derive(Debug, Clone)]
struct LoseMoneyStrategy {
    pub id: StrategyId,
}

impl Default for LoseMoneyStrategy {
    fn default() -> Self {
        Self {
            id: StrategyId::new("LoseMoneyStrategy"),
        }
    }
}

impl AlgoStrategy for LoseMoneyStrategy {
    type State = EngineState<DefaultGlobalData, LoseMoneyInstrumentData>;

    fn generate_algo_orders(
        &self,
        state: &Self::State,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>>,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>>,
    ) {
        let opens = state
            .instruments
            .instruments(&InstrumentFilter::None)
            .filter_map(|state| {
                let trade_not_sent_as_order_open = state.data.last_trade.as_ref()?;

                Some(OrderRequestOpen {
                    key: OrderKey {
                        exchange: state.instrument.exchange,
                        instrument: state.key,
                        strategy: self.id.clone(),
                        cid: ClientOrderId::random(),
                    },
                    state: RequestOpen {
                        side: Side::Buy,
                        price: Decimal::from_f64(trade_not_sent_as_order_open.price).unwrap(),
                        quantity: Decimal::from_f64(trade_not_sent_as_order_open.amount).unwrap(),
                        kind: OrderKind::Market,
                        time_in_force: TimeInForce::ImmediateOrCancel,
                    },
                })
            });

        (std::iter::empty(), opens)
    }
}

impl ClosePositionsStrategy for LoseMoneyStrategy {
    type State = EngineState<DefaultGlobalData, LoseMoneyInstrumentData>;

    fn close_positions_requests<'a>(
        &'a self,
        state: &'a Self::State,
        filter: &'a InstrumentFilter,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>> + 'a,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>> + 'a,
    )
    where
        ExchangeIndex: 'a,
        AssetIndex: 'a,
        InstrumentIndex: 'a,
    {
        close_open_positions_with_market_orders(&self.id, state, filter, |_| {
            ClientOrderId::random()
        })
    }
}

impl
    OnDisconnectStrategy<
        HistoricalClock,
        EngineState<DefaultGlobalData, LoseMoneyInstrumentData>,
        MultiExchangeTxMap,
        DefaultRiskManager<EngineState<DefaultGlobalData, LoseMoneyInstrumentData>>,
    > for LoseMoneyStrategy
{
    type OnDisconnect = ();

    fn on_disconnect(
        _: &mut Engine<
            HistoricalClock,
            EngineState<DefaultGlobalData, LoseMoneyInstrumentData>,
            MultiExchangeTxMap,
            Self,
            DefaultRiskManager<EngineState<DefaultGlobalData, LoseMoneyInstrumentData>>,
        >,
        _: ExchangeId,
    ) -> Self::OnDisconnect {
    }
}

impl
    OnTradingDisabled<
        HistoricalClock,
        EngineState<DefaultGlobalData, LoseMoneyInstrumentData>,
        MultiExchangeTxMap,
        DefaultRiskManager<EngineState<DefaultGlobalData, LoseMoneyInstrumentData>>,
    > for LoseMoneyStrategy
{
    type OnTradingDisabled = ();

    fn on_trading_disabled(
        _: &mut Engine<
            HistoricalClock,
            EngineState<DefaultGlobalData, LoseMoneyInstrumentData>,
            MultiExchangeTxMap,
            Self,
            DefaultRiskManager<EngineState<DefaultGlobalData, LoseMoneyInstrumentData>>,
        >,
    ) -> Self::OnTradingDisabled {
    }
}

#[derive(Debug, Clone)]
struct LoseMoneyInstrumentData {
    last_trade: Option<PublicTrade>,
    market_data: DefaultInstrumentMarketData,
}

impl Default for LoseMoneyInstrumentData {
    fn default() -> Self {
        Self {
            last_trade: None,
            market_data: DefaultInstrumentMarketData::default(),
        }
    }
}

impl InstrumentDataState for LoseMoneyInstrumentData {
    type MarketEventKind = DataKind;

    fn price(&self) -> Option<Decimal> {
        self.market_data.price()
    }
}

impl Processor<&MarketEvent<InstrumentIndex>> for LoseMoneyInstrumentData {
    type Audit = ();

    fn process(&mut self, event: &MarketEvent<InstrumentIndex>) -> Self::Audit {
        if let DataKind::Trade(trade) = &event.kind {
            self.last_trade = Some(trade.clone())
        } else {
            self.last_trade = None;
        }
    }
}

impl Processor<&AccountEvent> for LoseMoneyInstrumentData {
    type Audit = ();

    fn process(&mut self, _: &AccountEvent) -> Self::Audit {}
}

impl InFlightRequestRecorder for LoseMoneyInstrumentData {
    fn record_in_flight_cancel(&mut self, _: &OrderRequestCancel<ExchangeIndex, InstrumentIndex>) {}

    fn record_in_flight_open(&mut self, _: &OrderRequestOpen<ExchangeIndex, InstrumentIndex>) {}
}

fn args_constant(
    instruments: Vec<InstrumentConfig>,
    executions: Vec<ExecutionConfig>,
) -> Arc<
    BacktestArgsConstant<
        MarketDataInMemory<DataKind>,
        Daily,
        EngineState<DefaultGlobalData, LoseMoneyInstrumentData>,
    >,
> {
    // Construct IndexedInstruments
    let instruments = IndexedInstruments::new(instruments);

    // Initialise MarketData
    let market_events = market_data_from_file(FILE_PATH_MARKET_DATA_INDEXED);
    let market_data = MarketDataInMemory::new(Arc::new(market_events));
    let time_engine_start = DateTime::<Utc>::from_str("2025-03-25T23:07:00.773674205Z").unwrap();

    // Construct EngineState
    let engine_state = EngineStateBuilder::new(&instruments, DefaultGlobalData::default(), |_| {
        LoseMoneyInstrumentData::default()
    })
    .time_engine_start(time_engine_start)
    .trading_state(TradingState::Enabled)
    .build();

    Arc::new(BacktestArgsConstant {
        instruments,
        executions,
        market_data,
        summary_interval: Daily,
        engine_state,
    })
}

pub fn market_data_from_file<InstrumentKey, Kind>(
    file_path: &str,
) -> Vec<MarketStreamEvent<InstrumentKey, Kind>>
where
    InstrumentKey: for<'de> Deserialize<'de>,
    Kind: for<'de> Deserialize<'de>,
{
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file);

    reader
        .lines()
        .map(|line_result| {
            let line = line_result.unwrap();
            serde_json::from_str::<MarketStreamEvent<InstrumentKey, Kind>>(&line).unwrap()
        })
        .collect()
}

fn args_dynamic(
    risk_free_return: Decimal,
) -> BacktestArgsDynamic<
    LoseMoneyStrategy,
    DefaultRiskManager<EngineState<DefaultGlobalData, LoseMoneyInstrumentData>>,
> {
    BacktestArgsDynamic {
        id: SmolStr::new("benches/backtest"),
        risk_free_return,
        strategy: LoseMoneyStrategy::default(),
        risk: DefaultRiskManager::default(),
    }
}
