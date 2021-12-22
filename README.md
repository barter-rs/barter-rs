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

    // Create termination channels
    let (engine_termination_tx, engine_termination_rx) = oneshot::channel();    // Engine graceful remote shutdown
    let (traders_termination_tx, _) = broadcast::channel(1);                    // Shutdown channel to broadcast remote shutdown to every Trader instance
    
    // Create EventSink channel to listen to all Engine Events in real-time
    let (event_tx, event_rx) = unbounded_channel();
    let event_sink = EventSink::new();

    // Build global shared-state MetaPortfolio
    let portfolio = Arc::new(Mutex::new(
        MetaPortfolio::builder()
            .id(Uuid::new_v4())
            .starting_cash(10_000.0)
            .repository(InMemoryRepository::new())
            .allocation_manager(DefaultAllocator { default_order_value: 100.0 })
            .risk_manager(DefaultRisk {})
            .build_and_init()
            .expect("failed to build & initialise MetaPortfolio"),
    ));
    
    // Build Trader(s)
    let mut traders = Vec::new();
    traders.push(
        Trader::builder()
            .termination_rx(traders_termination_tx.subscribe())
            .event_sink(event_sink.clone())
            .portfolio(Arc::clone(&portfolio))
            .data(HistoricCandleHandler::new(HistoricDataLego { 
                exchange: "Binance".to_string(),
                symbol: "btcusdt".to_string(),
                candles: vec![Candle::default()].into_iter(),
            })
            .strategy(RSIStrategy::new(StrategyConfig { rsi_period: 14 }))
            .execution(SimulatedExecution::new(ExecutionConfig {
                simulates_fees_pct: Fees {
                    exchange: 0.1,
                    slippage: 0.05,
                    network: 0.0,}
            }))
            .build()
            .expect("failed to build trader")
    );
    
    // Build Engine
    let engine = Engine::builder()
        .termination_rx(engine_termination_rx)
        .traders_termination_tx(traders_termination_tx)
        .statistics(TradingSummary::new(&config.statistics))
        .portfolio(portfolio)
        .traders(traders)
        .build()
        .expect("failed to build engine");
        
    // Listen to Engine Events & do something with them
    tokio::spawn(listen_to_events(event_rx));
        
    // --- Run Trading Session Until Remote Shutdown ---
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
* [`Barter-Data`]: High performance & normalised WebSocket intergration for leading cryptocurrency exchanges - batteries 
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