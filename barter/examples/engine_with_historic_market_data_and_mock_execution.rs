use barter::{
    engine::{
        audit::Audit,
        clock::{EngineClock, HistoricalClock},
        command::Command,
        run,
        state::{
            generate_empty_indexed_engine_state,
            instrument::{filter::InstrumentFilter, market_data::DefaultMarketData},
            trading::TradingState,
        },
        Engine,
    },
    execution::builder::ExecutionBuilder,
    logging::init_logging,
    risk::{DefaultRiskManager, DefaultRiskManagerState},
    statistic::time::Daily,
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
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::{debug, info, warn};

const EXCHANGE: ExchangeId = ExchangeId::BinanceSpot;
const RISK_FREE_RETURN: Decimal = dec!(0.05);
const MOCK_EXCHANGE_ROUND_TRIP_LATENCY_MS: u64 = 100;
const MOCK_EXCHANGE_FEES_PERCENT: Decimal = dec!(0.05);
const MOCK_EXCHANGE_STARTING_BALANCE_USD: Decimal = dec!(10_000.0);

const FILE_PATH_HISTORIC_TRADES_AND_L1S: &str =
    "barter/examples/data/binance_spot_market_data_with_disconnect_events.json";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise Tracing
    init_logging();

    // Initialise Channels
    let (feed_tx, mut feed_rx) = mpsc_unbounded();
    let (audit_tx, audit_rx) = mpsc_unbounded();

    // Construct IndexedInstruments
    let instruments = IndexedInstruments::new(unindexed_instruments());

    // Initialise HistoricalClock & MarketStream
    let (clock, market_stream) =
        init_historic_clock_and_market_data_stream(FILE_PATH_HISTORIC_TRADES_AND_L1S);

    // Forward market data events to Engine feed
    tokio::spawn(market_stream.forward_to(feed_tx.clone()));

    // Define initial mock AccountSnapshot
    let initial_account =
        build_initial_account_snapshot(&instruments, MOCK_EXCHANGE_STARTING_BALANCE_USD);

    // Initialise ExecutionManager & forward Account Streams to Engine feed
    let (execution_txs, account_stream) = ExecutionBuilder::new(&instruments)
        .add_mock(MockExecutionConfig::new(
            EXCHANGE,
            initial_account,
            MOCK_EXCHANGE_ROUND_TRIP_LATENCY_MS,
            MOCK_EXCHANGE_FEES_PERCENT,
        ))?
        .init()
        .await?;
    tokio::spawn(account_stream.forward_to(feed_tx.clone()));

    // Construct empty EngineState from IndexedInstruments
    let state = generate_empty_indexed_engine_state::<DefaultMarketData, _, _>(
        // Note: you may want to start to engine with TradingState::Disabled and turn on later
        TradingState::Enabled,
        &instruments,
        clock.time(),
        DefaultStrategyState,
        DefaultRiskManagerState,
    );

    // Construct Engine
    let mut engine = Engine::new(
        clock,
        state,
        execution_txs,
        DefaultStrategy::default(),
        DefaultRiskManager::default(),
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

    // Run dummy asynchronous AuditStream consumer
    // Note: you probably want to use this Stream to replicate EngineState, or persist events, etc.
    //  --> eg/ see examples/engine_with_replica_engine_state.rs
    let audit_task = tokio::spawn(async move {
        let mut audit_stream = audit_rx.into_stream();
        while let Some(audit) = audit_stream.next().await {
            debug!(?audit, "AuditStream consumed AuditTick");
            if let Audit::Shutdown(_) = audit.event {
                break;
            }
        }
        audit_stream
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

    // Await Engine & AuditStream task graceful shutdown
    // Note: Engine & AuditStream returned, ready for further use
    let (engine, _shutdown_audit) = engine_task.await?;
    let _audit_stream = audit_task.await?;

    // Generate TradingSummary<Daily>
    let trading_summary = engine
        .trading_summary_generator(RISK_FREE_RETURN)
        .generate(Daily);

    // Print TradingSummary<Daily> to terminal (could save in a file, send somewhere, etc.)
    trading_summary.print_summary();

    Ok(())
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
fn init_historic_clock_and_market_data_stream(
    file_path: &str,
) -> (
    HistoricalClock,
    impl Stream<Item = MarketStreamEvent<InstrumentIndex, DataKind>>,
) {
    let data = std::fs::read_to_string(file_path).unwrap();
    let events =
        serde_json::from_str::<Vec<MarketStreamResult<InstrumentIndex, DataKind>>>(&data).unwrap();

    let time_exchange_first = events
        .iter()
        .find_map(|result| match result {
            MarketStreamResult::Item(Ok(event)) => Some(event.time_exchange),
            _ => None,
        })
        .unwrap();

    let clock = HistoricalClock::new(time_exchange_first);

    let stream = stream::iter(events)
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
        });

    (clock, stream)
}

fn build_initial_account_snapshot(
    instruments: &IndexedInstruments,
    balance_usd: Decimal,
) -> UnindexedAccountSnapshot {
    let balances = instruments
        .assets()
        .iter()
        .map(|keyed_asset| {
            AssetBalance::new(
                keyed_asset.value.asset.name_exchange.clone(),
                if keyed_asset.value.asset.name_internal.as_ref() == "usdt" {
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
