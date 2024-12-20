use barter::{
    engine::{
        audit::{manager::AuditManager, Auditor},
        command::Command,
        run,
        state::{
            asset::generate_empty_indexed_asset_states,
            connectivity::generate_empty_indexed_connectivity_states,
            instrument::{
                generate_empty_indexed_instrument_states, manager::InstrumentFilter,
                market_data::DefaultMarketData,
            },
            trading::TradingState,
            EngineState,
        },
        Engine,
    },
    execution::builder::ExecutionBuilder,
    risk::{DefaultRiskManager, DefaultRiskManagerState},
    statistic::{summary::TradingSummaryGenerator, time::Daily},
    strategy::{DefaultStrategy, DefaultStrategyState},
    EngineEvent,
};
use barter_data::{
    event::DataKind,
    streams::{
        consumer::{MarketStreamEvent, MarketStreamResult},
        reconnect::{stream::ReconnectingStream, Event},
    },
};
use barter_execution::{
    balance::{AssetBalance, Balance},
    client::mock::MockExecutionConfig,
    InstrumentAccountSnapshot, UnindexedAccountSnapshot,
};
use barter_instrument::{
    asset::Asset,
    exchange::ExchangeId,
    index::IndexedInstruments,
    instrument::{
        kind::InstrumentKind,
        spec::{
            InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity,
            OrderQuantityUnits,
        },
        Instrument, InstrumentIndex,
    },
    Underlying,
};
use barter_integration::channel::{mpsc_unbounded, ChannelTxDroppable, Tx};
use chrono::Utc;
use futures::{stream, Stream, StreamExt};
use rust_decimal_macros::dec;
use tracing::{info, warn};

const EXCHANGE: ExchangeId = ExchangeId::BinanceSpot;
const RISK_FREE_RETURN: f64 = 0.05;

const FILE_PATH_HISTORIC_TRADES_AND_L1S: &str =
    "barter/examples/data/binance_spot_market_data.json";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise Tracing
    init_logging();

    // Initialise Channels
    let (feed_tx, mut feed_rx) = mpsc_unbounded();
    let (audit_tx, mut audit_rx) = mpsc_unbounded();

    // Construct IndexedInstruments
    let instruments = IndexedInstruments::new(unindexed_instruments());

    // Forward historical market data events to Engine feed
    let market_stream = init_historic_market_data_stream(FILE_PATH_HISTORIC_TRADES_AND_L1S);
    tokio::spawn(market_stream.forward_to(feed_tx.clone()));

    // Define initial mock AccountSnapshot
    let initial_account = build_initial_account_snapshot(&instruments, 10_000.0);

    // Initialise ExecutionManager & forward Account Streams to Engine feed
    let (execution_txs, account_stream) = ExecutionBuilder::new(&instruments)
        .add_mock(MockExecutionConfig::new(EXCHANGE, initial_account, 0.03))?
        .init()
        .await?;
    tokio::spawn(account_stream.forward_to(feed_tx.clone()));

    // Construct empty EngineState from IndexedInstruments
    let state = EngineState {
        trading: TradingState::Enabled,
        connectivity: generate_empty_indexed_connectivity_states(&instruments),
        assets: generate_empty_indexed_asset_states(&instruments),
        instruments: generate_empty_indexed_instrument_states::<DefaultMarketData>(&instruments),
        strategy: DefaultStrategyState,
        risk: DefaultRiskManagerState,
    };

    // Construct Engine
    let mut engine = Engine::new(
        || Utc::now(),
        state.clone(),
        execution_txs,
        DefaultStrategy::default(),
        DefaultRiskManager::default(),
    );

    // Construct AuditManager w/ initial EngineState
    let mut audit_manager = AuditManager::new(
        engine.audit(state),
        TradingSummaryGenerator::init(
            RISK_FREE_RETURN,
            engine.time(),
            &engine.state.instruments,
            &engine.state.assets,
        ),
    );

    // Run synchronous Engine on blocking task
    let engine_task = tokio::task::spawn_blocking(move || {
        let shutdown_audit = run(
            &mut feed_rx,
            &mut engine,
            &mut ChannelTxDroppable::new(audit_tx),
        );
        (engine, shutdown_audit)
    });

    // Run synchronous AuditManager on blocking task
    let audit_task = tokio::task::spawn_blocking(move || {
        audit_manager.run(&mut audit_rx).unwrap();
        (audit_manager, audit_rx)
    });

    // Let the example run for 4 seconds..., then:
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    // 1. Disable Strategy order generation (still continues to update EngineState)
    feed_tx.send(TradingState::Disabled)?;
    // 2. Cancel all open orders
    feed_tx.send(Command::CancelOrders(InstrumentFilter::None))?;
    // 3. Send orders to close current positions
    feed_tx.send(Command::ClosePositions(InstrumentFilter::None))?;
    // 4. Stop Engine run loop
    feed_tx.send(EngineEvent::Shutdown)?;

    // Await Engine & AuditManager graceful shutdown
    let (_engine, shutdown_audit) = engine_task.await?;
    info!(?shutdown_audit, "Engine shutdown");
    let (audit_manager, _audit_stream) = audit_task.await?;
    info!("AuditManager shutdown");

    // Generate TradingSummary
    let trading_summary = audit_manager.summary.generate(Daily);

    // Print TradingSummary<Daily> to terminal (could save in a file, send somewhere, etc)
    trading_summary.print_summary();

    Ok(())
}

fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the INFO
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))
        // Enable Json formatting
        .json()
        // Install this Tracing subscriber as global default
        .init()
}

fn unindexed_instruments() -> Vec<Instrument<ExchangeId, Asset>> {
    vec![
        Instrument::new(
            EXCHANGE,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            InstrumentKind::Spot,
            InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.0001), dec!(0.0)),
                InstrumentSpecQuantity::new(
                    OrderQuantityUnits::Quote,
                    dec!(0.00001),
                    dec!(0.00001),
                ),
                InstrumentSpecNotional::new(dec!(5.0)),
            ),
        ),
        Instrument::new(
            EXCHANGE,
            "binance_spot_eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            InstrumentKind::Spot,
            InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.0001), dec!(0.0001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            ),
        ),
        Instrument::new(
            EXCHANGE,
            "binance_spot_sol_usdt",
            "SOLUSDT",
            Underlying::new("sol", "usdt"),
            InstrumentKind::Spot,
            InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.001), dec!(0.001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            ),
        ),
    ]
}

// Note that there are far more intelligent ways of streaming historical market data, this is
// just for demonstration purposes.
//
// For example:
// - Stream from database
// - Stream from file
fn init_historic_market_data_stream(
    file_path: &str,
) -> impl Stream<Item = MarketStreamEvent<InstrumentIndex, DataKind>> {
    let data = std::fs::read_to_string(file_path).unwrap();
    let events =
        serde_json::from_str::<Vec<MarketStreamResult<InstrumentIndex, DataKind>>>(&data).unwrap();

    stream::iter(events)
        .with_error_handler(|error| warn!(?error, "MarketStream generated error"))
        .inspect(|event| match event {
            Event::Reconnecting(exchange) => {
                info!(%exchange, "sending historical disconnection to Engine")
            }
            Event::Item(event) => {
                info!(
                    exchange = %event.exchange,
                    instrument = %event.instrument,
                    kind = event.kind.kind_name(),
                    "sending historical event to Engine"
                )
            }
        })
}

fn build_initial_account_snapshot(
    instruments: &IndexedInstruments,
    balance_usd: f64,
) -> UnindexedAccountSnapshot {
    let balances = instruments
        .assets()
        .iter()
        .map(|keyed_asset| {
            AssetBalance::new(
                keyed_asset.value.asset.name_exchange.clone(),
                if keyed_asset.value.asset.name_internal.as_ref() == "usd" {
                    Balance::new(balance_usd, balance_usd)
                } else {
                    Balance::default()
                },
                Utc::now(),
            )
        })
        .collect();

    let instruments = instruments
        .instruments()
        .iter()
        .map(|keyed_instrument| {
            InstrumentAccountSnapshot::new(keyed_instrument.value.name_exchange.clone(), vec![])
        })
        .collect();

    UnindexedAccountSnapshot {
        balances,
        instruments,
    }
}
