# Jackbot
Jackbot core is a Rust framework for building high-performance live-trading, paper-trading and back-testing systems.
* **Fast**: Written in native Rust. Minimal allocations. Data-oriented state management system with direct index lookups.
* **Robust**: Strongly typed. Thread safe. Extensive test coverage.
* **Customisable**: Plug and play Strategy and RiskManager components that facilitates most trading strategies (MarketMaking, StatArb, HFT, etc.).
* **Scalable**: Multithreaded architecture with modular design. Leverages Tokio for I/O. Memory efficient data structures.

## Overview
Jackbot core is a Rust framework for building professional grade live-trading, paper-trading and back-testing systems. The
central Engine facilitates executing on many exchanges simultaneously, and offers the flexibility to run most types of
trading strategies. It allows turning algorithmic order generation on/off and can action Commands issued from external
processes (eg/ CloseAllPositions, OpenOrders, CancelOrders, etc.)

At a high-level, it provides a few major components:
* `SystemBuilder` for constructing and initialising a full trading `System`.
* `Engine` with plug and play `Strategy` and `RiskManager` components.
* Centralised cache friendly `EngineState` management with O(1) constant lookups using indexed data structures.  
* `Strategy` interfaces for customising Engine behavior (AlgoStrategy, ClosePositionsStrategy, OnDisconnectStrategy, etc.).
* `RiskManager` interface for defining custom risk logic which checking generated algorithmic orders.
* Event-driven system that allows for Commands to be issued from external processes (eg/ CloseAllPositions, OpenOrders, CancelOrders, etc.),
  as well as turning algorithmic trading on/off.
* Comprehensive statistics package that provides a summary of key performance metrics (PnL, Sharpe, Sortino, Drawdown, etc.).


## Examples

#### Paper Trading With Live Market Data & Mock Execution

```rust,no_run
const FILE_PATH_SYSTEM_CONFIG: &str = "Jackbot/examples/config/system_config.json";
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
    );

    // Build & run full system:
    let mut system = SystemBuilder::new(args)
        // Engine feed in Sync mode (Iterator input)
        .engine_feed_mode(EngineFeedMode::Iterator)

        // Engine starts with algorithmic trading disabled
        .trading_state(TradingState::Disabled)

        // Build System, but don't start spawning tasks yet
        .build::<EngineEvent, DefaultGlobalData, DefaultInstrumentMarketData>()?

        // Init System, spawning component tasks on the current runtime
        .init_with_runtime(tokio::runtime::Handle::current())
        .await?;

    // Enable trading
    system.trading_state(TradingState::Enabled);

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
```