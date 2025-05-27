use barter::{
    EngineEvent,
    engine::{
        clock::HistoricalClock,
        state::{
            global::DefaultGlobalData,
            instrument::{data::DefaultInstrumentMarketData, filter::InstrumentFilter},
            trading::TradingState,
        },
    },
    logging::init_logging,
    risk::DefaultRiskManager,
    statistic::time::Daily,
    strategy::DefaultStrategy,
    system::{
        builder::{AuditMode, EngineFeedMode, SystemArgs, SystemBuilder},
        config::SystemConfig,
    },
};
use barter_data::{
    event::DataKind,
    streams::{
        consumer::{MarketStreamEvent, MarketStreamResult},
        reconnect::{Event, stream::ReconnectingStream},
    },
};
use barter_instrument::{index::IndexedInstruments, instrument::InstrumentIndex};
use futures::{Stream, StreamExt, stream};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::{fs::File, io::BufReader, time::Duration};
use tracing::{info, warn};

const FILE_PATH_SYSTEM_CONFIG: &str = "barter/examples/config/system_config.json";
const FILE_PATH_HISTORIC_MARKET_EVENTS: &str =
    "barter/examples/data/binance_spot_market_data_with_disconnect_events.json";
const RISK_FREE_RETURN: Decimal = dec!(0.05);

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

    // Initialise HistoricalClock & MarketStream
    let (clock, market_stream) =
        init_historic_clock_and_market_stream(FILE_PATH_HISTORIC_MARKET_EVENTS);

    // Construct SystemArgs
    let args = SystemArgs::new(
        &instruments,
        executions,
        clock,
        DefaultStrategy::default(),
        DefaultRiskManager::default(),
        market_stream,
        DefaultGlobalData::default(),
        |_| DefaultInstrumentMarketData::default(),
    );

    // Build & run full system:
    // See SystemBuilder for all configuration options
    let system = SystemBuilder::new(args)
        // Engine feed in Async mode (Stream input)
        .engine_feed_mode(EngineFeedMode::Stream)
        // Audit feed is disabled (Engine does not send audits)
        .audit_mode(AuditMode::Disabled)
        // Engine starts with TradingState::Enabled
        .trading_state(TradingState::Enabled)
        // Build System, but don't start spawning tasks yet
        .build::<EngineEvent, _>()?
        // Init System, spawning component tasks on the current runtime
        .init_with_runtime(tokio::runtime::Handle::current())
        .await?;

    // Let the example run for 5 seconds...
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Before shutting down, CancelOrders and then ClosePositions
    system.cancel_orders(InstrumentFilter::None);
    system.close_positions(InstrumentFilter::None);

    // Shutdown
    let (engine, _shutdown_audit) = system.shutdown().await?;

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

// Note that there are far more intelligent ways of streaming historical market data, this is
// just for demonstration purposes.
//
// For example:
// - Stream from database
// - Stream from file (more efficiently)
fn init_historic_clock_and_market_stream(
    file_path: &str,
) -> (
    HistoricalClock,
    impl Stream<Item = MarketStreamEvent<InstrumentIndex, DataKind>> + use<>,
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
