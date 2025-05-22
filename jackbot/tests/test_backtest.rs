use Jackbot::{
    backtest::{backtest, run_backtests, BacktestArgsConstant, BacktestArgsDynamic, market_data::MarketDataInMemory},
    engine::{state::{EngineState, builder::EngineStateBuilder, global::DefaultGlobalData, instrument::data::DefaultInstrumentMarketData, trading::TradingState}},
    risk::DefaultRiskManager,
    strategy::DefaultStrategy,
    statistic::time::Daily,
    system::config::ExecutionConfig,
};
use jackbot_data::{event::{MarketEvent, DataKind}, streams::consumer::MarketStreamEvent, subscription::trade::PublicTrade};
use jackbot_execution::{client::mock::MockExecutionConfig, AccountSnapshot};
use jackbot_instrument::{exchange::ExchangeId, index::IndexedInstruments, instrument::{Instrument, InstrumentIndex}, Side, Underlying};
use rust_decimal::Decimal;
use chrono::Utc;
use smol_str::SmolStr;
use std::sync::Arc;

#[tokio::test]
async fn test_backtest_runs_with_default_strategy() {
    // setup single instrument
    let instruments = IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .build();

    // single trade market event
    let event = MarketStreamEvent::Item(MarketEvent {
        time_exchange: Utc::now(),
        time_received: Utc::now(),
        exchange: ExchangeId::BinanceSpot,
        instrument: InstrumentIndex(0),
        kind: DataKind::Trade(PublicTrade { id: "1".into(), price: 100.0, amount: 1.0, side: Side::Buy }),
    });
    let market_data = MarketDataInMemory::new(Arc::new(vec![event]));
    let time_engine_start = market_data.time_first_event().await.unwrap();

    // engine state
    let engine_state: EngineState<DefaultGlobalData, DefaultInstrumentMarketData> =
        EngineStateBuilder::new(&instruments, DefaultGlobalData::default(), DefaultInstrumentMarketData::default)
            .time_engine_start(time_engine_start)
            .trading_state(TradingState::Enabled)
            .build();

    // mock execution config
    let execution_snapshot = AccountSnapshot { exchange: ExchangeId::BinanceSpot, balances: Vec::new(), instruments: Vec::new() };
    let executions = vec![ExecutionConfig::Mock(MockExecutionConfig { mocked_exchange: ExchangeId::BinanceSpot, initial_state: execution_snapshot, latency_ms: 0, fees_percent: Decimal::ZERO })];

    let args_constant = Arc::new(BacktestArgsConstant {
        instruments,
        executions,
        market_data,
        summary_interval: Daily,
        engine_state,
    });

    let args_dynamic = BacktestArgsDynamic {
        id: SmolStr::new("test"),
        risk_free_return: Decimal::ZERO,
        strategy: DefaultStrategy::<EngineState<DefaultGlobalData, DefaultInstrumentMarketData>>::default(),
        risk: DefaultRiskManager::<EngineState<DefaultGlobalData, DefaultInstrumentMarketData>>::default(),
    };

    let summary = backtest(args_constant, args_dynamic).await.expect("backtest");
    assert_eq!(summary.id, SmolStr::new("test"));
}

#[tokio::test]
async fn test_run_backtests_multiple() {
    let instruments = IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .build();

    let event = MarketStreamEvent::Item(MarketEvent {
        time_exchange: Utc::now(),
        time_received: Utc::now(),
        exchange: ExchangeId::BinanceSpot,
        instrument: InstrumentIndex(0),
        kind: DataKind::Trade(PublicTrade { id: "1".into(), price: 100.0, amount: 1.0, side: Side::Buy }),
    });
    let market_data = MarketDataInMemory::new(Arc::new(vec![event]));
    let time_engine_start = market_data.time_first_event().await.unwrap();

    let engine_state: EngineState<DefaultGlobalData, DefaultInstrumentMarketData> =
        EngineStateBuilder::new(&instruments, DefaultGlobalData::default(), DefaultInstrumentMarketData::default)
            .time_engine_start(time_engine_start)
            .trading_state(TradingState::Enabled)
            .build();

    let execution_snapshot = AccountSnapshot { exchange: ExchangeId::BinanceSpot, balances: Vec::new(), instruments: Vec::new() };
    let executions = vec![ExecutionConfig::Mock(MockExecutionConfig { mocked_exchange: ExchangeId::BinanceSpot, initial_state: execution_snapshot, latency_ms: 0, fees_percent: Decimal::ZERO })];

    let args_constant = Arc::new(BacktestArgsConstant {
        instruments,
        executions,
        market_data,
        summary_interval: Daily,
        engine_state,
    });

    let dynamic = BacktestArgsDynamic {
        id: SmolStr::new("a"),
        risk_free_return: Decimal::ZERO,
        strategy: DefaultStrategy::<EngineState<DefaultGlobalData, DefaultInstrumentMarketData>>::default(),
        risk: DefaultRiskManager::<EngineState<DefaultGlobalData, DefaultInstrumentMarketData>>::default(),
    };
    let summaries = run_backtests(args_constant, vec![dynamic.clone(), dynamic]).await.expect("run");
    assert_eq!(summaries.num_backtests, 2);
}
