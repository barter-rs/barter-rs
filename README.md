# Barter
Barter is an open-source Rust framework for building **event-driven live-trading & backtesting systems**. 
Algorithmic trade with the peace of mind that comes from knowing your strategies have been
backtested with a near-identical trading Engine.
It is:
* **Fast**: Barter provides a multi-threaded trading Engine framework built in high-performance Rust (in-rust-we-trust).
* **Easy**: Barter provides a modularised data architecture that focuses on simplicity.
* **Customisable**: A set of traits define how every Barter component communicates, providing a highly extensible 
framework for trading.

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/barter.svg
[crates-url]: https://crates.io/crates/barter

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-rs/-/blob/main/LICENCE

[actions-badge]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-rs/badges/-/blob/main/pipeline.svg
[actions-url]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-rs/-/commits/master

[discord-badge]: https://img.shields.io/discord/910237311332151317.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/wE7RqhnQMV

[API Documentation] |
[Chat]

[`Barter`]: https://crates.io/crates/barter
[`Barter-Data`]: https://crates.io/crates/barter-data
[API Documentation]: https://docs.rs/barter/latest/barter/
[Chat]: https://discord.gg/wE7RqhnQMV

## Overview
Barter is an open-source Rust framework for building **event-driven live-trading & backtesting systems**. It provides 
a high-performance, easy to customise, trading Engine that enables backtesting strategies on a near-identical system 
to live trading. At a high level, it provides several de-coupled components that interact via a set of traits:

* **Data**: Continuer & MarketGenerator traits govern the generation of a MarketEvents data feed that acts as the system 
heartbeat. For example, a LiveCandleHandler implementation is provided utilising [`Barter-Data`]'s WebSocket functionality to
provide a live market Candle data feed to the system.
* **Strategy**: The SignalGenerator trait governs potential generation of SignalEvents after analysing incoming 
MarketEvents. SignalEvents are advisory signals sent to the Portfolio for analysis.
* **Portfolio**: MarketUpdater, OrderGenerator, and FillUpdater govern global state Portfolio implementations. A 
Portfolio may generate OrderEvents after receiving advisory SignalEvents from a Strategy. The Portfolio's state 
updates after receiving MarketEvents and FillEvents.
* **Execution**: The FillGenerator trait governs the generation of FillEvents after receiving OrderEvents from the 
Portfolio. For example, a SimulatedExecution handler implementation is provided for simulating any exchange execution
behaviour required in dry-trading or backtesting runs. 
* **Statistic**: Provides metrics such as Sharpe Ratio, Calmar Ratio, and Max Drawdown to analyse trading session 
performance. One-pass dispersion algorithms analyse each closed Position and efficiently calculates a trading summary.
* **Trader**: Capable of trading a single market pair using a customisable selection of it's own Data, Strategy & 
Execution instances, as well as shared access to a global Portfolio. 
* **Engine**: Multi-threaded trading Engine capable of trading with an arbitrary number of Trader market pairs. Each 
contained Trader instance operates on its own thread.

## Example
* **For brevity**: Imports are not included. 
* **For simplicity**: 
  * Engine and Trader(s) are configuration with hard-coded values rather than loaded in configuration values.
  * Remote shutdown is not orchestrated via the engine_termination_rx, this should be added as per 
  your taste (eg/ HTTP endpoint). 

```rust,no_run
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup Logger & Load Config For Engine & Trader Instances Here

    // Create channel to distribute Commands to the Engine & it's Traders (eg/ Command::Terminate)
    let (command_tx, command_rx) = mpsc::channel(20);
    
    // Create Event channel to listen to all Engine Events in real-time
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let event_tx = EventTx::new(event_tx);
    
    // Generate unique identifier to associate an Engine's components
    let engine_id = Uuid::new_v4();
    
    // Create the Market(s) to be traded on (1-to-1 relationship with a Trader) 
    let market = Market::new("binance", "btc_usdt".to_owned());
    
    // Build global shared-state MetaPortfolio (1-to-1 relationship with an Engine)
    let portfolio = Arc::new(Mutex::new(
        MetaPortfolio::builder()
            .engine_id(engine_id)
            .markets(vec![market.clone()])
            .starting_cash(10_000.0)
            .repository(InMemoryRepository::new())
            .allocation_manager(DefaultAllocator { default_order_value: 100.0 })
            .risk_manager(DefaultRisk {})
            .statistic_config(StatisticConfig { 
                starting_equity: 10_000.0, 
                trading_days_per_year: 365, 
                risk_free_return: 0.0 
            })
            .build_and_init()
            .expect("failed to build & initialise MetaPortfolio"),
    ));
    
    // Build Trader(s)
    let mut traders = Vec::new();
    
    // Create channel for each Trader so the Engine can distribute Commands to it
    let (trader_command_tx, trader_command_rx) = mpsc::channel(10);

    traders.push(
        Trader::builder()
            .engine_id(engine_id)
            .market(market.clone())
            .command_rx(trader_command_rx)
            .event_tx(event_tx.clone())
            .portfolio(Arc::clone(&portfolio))
            .data(HistoricCandleHandler::new(HistoricDataLego {
                exchange: "binance",
                symbol: "btcusdt".to_string(),
                candles: vec![test_util::candle()].into_iter()
            }))
            .strategy(RSIStrategy::new(StrategyConfig { rsi_period: 14 }))
            .execution(SimulatedExecution::new(ExecutionConfig {
                simulated_fees_pct: Fees {
                        exchange: 0.1,
                        slippage: 0.05,
                        network: 0.0,}
                }))
            .build()
            .expect("failed to build trader")
    );
    
    // Build Engine (1-to-many relationship with Traders)
    
    // Create HashMap<Market, trader_command_tx> so Engine can route Commands to Traders 
    let trader_command_txs = HashMap::from_iter([(market, trader_command_tx)]);
    
    let engine = Engine::builder()
        .engine_id(engine_id)
        .command_rx(command_rx)
        .portfolio(portfolio)
        .traders(traders)
        .trader_command_txs(trader_command_txs))
        .statistics_summary(TradingSummary::init(StatisticConfig {
            starting_equity: 1000.0,
            trading_days_per_year: 365,
            risk_free_return: 0.0
        })
        .build()
        .expect("failed to build engine");
        
    // Listen to Engine Events & do something with them
    tokio::spawn(listen_to_events(event_rx));
        
    // --- Run Trading Session Until Remote Shutdown OR Data Feed ends naturally (ie/ backtest) ---
    engine.run().await;
}
```

## Getting Help
Firstly, see if the answer to your question can be found in the [API Documentation]. If the answer is not there, I'd be
happy to help to [Chat] and try answer your question via Discord.

## Contributing
:tada: Thanks for your help in improving the barter ecosystem! Please do get in touch on the discord to discuss
development, new features, and the future roadmap.

## Related Projects
In addition to the Barter crate, the Barter project also maintains:
* [`Barter-Data`]: High performance & normalised WebSocket integration for leading cryptocurrency exchanges - batteries 
included.

## Roadmap
* Build & integrate the new "Barter-Execution" library to provide additional out-of-the-box functionality for executing
OrderEvents.
* Build Strategy utilities for aggregating tick-by-tick Trades and/or Candles into multi-timeframe datastructures, as well
as providing an example multi-timeframe Strategy using the utilities.  
* Flesh out the MetaPortfolio Allocator & Risk management systems.
* And more!

## Licence
This project is licensed under the [MIT license].

[MIT license]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-rs/-/blob/master/LICENSE

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.