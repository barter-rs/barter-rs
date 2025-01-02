# Barter
Barter is an algorithmic trading ecosystem of Rust libraries for building high-performance live-trading, paper-trading 
and back-testing systems.
* **Fast**: Written in native Rust. Minimal allocations. Data-oriented state management system with direct index lookups.
* **Robust**: Strongly typed. Thread safe. Extensive test coverage.
* **Customisable**: Plug and play Strategy and RiskManager components that facilitates most trading strategies (MarketMaking, StatArb, HFT, etc.).
* **Scalable**: Multithreaded architecture with modular design. Leverages Tokio for I/O. Memory efficient data structures.  

**See: [`Barter`], [`Barter-Data`], [`Barter-Instrument`], [`Barter-Execution`] & [`Barter-Integration`] for 
comprehensive documentation and examples for each library.**

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/barter.svg
[crates-url]: https://crates.io/crates/barter

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/barter-rs/barter-rs/blob/develop/LICENSE

[discord-badge]: https://img.shields.io/discord/910237311332151317.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/wE7RqhnQMV

[`Barter`]: https://crates.io/crates/barter
[`Barter-Instrument`]: https://crates.io/crates/barter-instrument
[`Barter-Data`]: https://crates.io/crates/barter-data
[`Barter-Execution`]: https://crates.io/crates/barter-execution
[`Barter-Integration`]: https://crates.io/crates/barter-integration
[API Documentation]: https://docs.rs/barter/latest/barter/
[Chat]: https://discord.gg/wE7RqhnQMV

## Overview
Barter is an algorithmic trading ecosystem of Rust libraries for building high-performance live-trading, paper-trading 
and back-testing systems. It is made up of several easy-to-use, extensible crates:
* **Barter**: Algorithmic trading Engine with feature rich state management system.
* **Barter-Instrument**: Exchange, Instrument and Asset data structures and utilities. 
* **Barter-Data**: Stream public market data from financial venues. Easily extensible via the MarketStream interface.
* **Barter-Execution**: Stream private account data and execute orders. Easily extensible via the ExecutionClient interface. 
* **Barter-Integration**: Low-level frameworks for flexible REST/WebSocket integrations.

## Notable Features
- Stream public market data from financial venues via the [`Barter-Data`] library. 
- Stream private account data, execute orders (live or mock)** via the [`Barter-Execution`] library.
- Plug and play Strategy and RiskManager components that facilitate most trading strategies. 
- Flexible Engine that facilitates trading strategies that execute on many exchanges simultaneously.
- Use mock MarketStream or Execution components to enable back-testing on a near-identical trading system as live-trading.  
- Centralised cache friendly state management system with O(1) constant lookups using indexed data structures.
- Robust Order management system - use stand-alone or with Barter. 
- Trading summaries with comprehensive performance metrics (PnL, Sharpe, Sortino, Drawdown, etc.).
- Turn on/off algorithmic trading from an external process (eg/ UI, Telegram, etc.) whilst still processing market/account data. 
- Issue Engine Commands from an external process (eg/ UI, Telegram, etc.) to initiate actions (CloseAllPositions, OpenOrders, CancelOrders, etc.).
- EngineState replica manager that processes the Engine AuditStream to facilitate non-hot path monitoring components (eg/ UI, Telegram, etc.).

[barter-examples]: https://github.com/barter-rs/barter-rs/tree/develop/barter/examples

## Examples
* See [here][barter-examples] for the compilable example including imports.
* See sub-crates for further examples of each library.

#### Engine With Live Market Data & Mock Execution

```rust,no_run
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

    // Construct EngineState from IndexedInstruments and hard-coded exchange asset Balances
    let state =
        EngineState::<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>::builder(
            &instruments,
        )
        .time_engine_start(clock.time())
        // Note: you may want to start to engine with TradingState::Disabled and turn on later
        .trading_state(TradingState::Enabled)
        .balances([
            (EXCHANGE, "usdt", STARTING_BALANCE_USDT),
            (EXCHANGE, "btc", STARTING_BALANCE_BTC),
            (EXCHANGE, "eth", STARTING_BALANCE_ETH),
            (EXCHANGE, "sol", STARTING_BALANCE_SOL),
        ])
        // Note: can add other initial data via this builder (eg/ exchange asset balances)
        .build();

    // Generate initial AccountSnapshot from EngineState for BinanceSpot MockExchange
    // Note: for live-trading this would be automatically fetched via the AccountStream init
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
            if let EngineAudit::Shutdown(_) = audit.event {
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

fn indexed_instruments() -> IndexedInstruments {
    IndexedInstruments::builder()
        .add_instrument(Instrument::new(
            EXCHANGE,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            InstrumentKind::Spot,
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
        .add_instrument(Instrument::new(
            EXCHANGE,
            "binance_spot_eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            InstrumentKind::Spot,
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.0001), dec!(0.0001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .add_instrument(Instrument::new(
            EXCHANGE,
            "binance_spot_sol_usdt",
            "SOLUSDT",
            Underlying::new("sol", "usdt"),
            InstrumentKind::Spot,
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.001), dec!(0.001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .build()
}
```

## Getting Help
Firstly, see if the answer to your question can be found in the [API Documentation]. If the answer is not there, I'd be
happy to help via [Chat] and try answer your question via Discord.

## Support Barter Development
Help us advance Barter's capabilities by becoming a sponsor (or supporting me with a tip!).

Your contribution will allow me to dedicate more time to Barter, accelerating feature development and improvements.

**Please email *justastream.code@gmail.com* for all inquiries**

### Sponsorship Tiers
* 🥇 **Sponsor** - Your name, logo, and website link will be displayed below.
* 🥈 **Supporter** - Your name listed as supporter.

### Current Sponsors
*Your name, logo and website link could be here*

### Current Supporters
*Your name could be here*

---
**Thank you to all our sponsors and supporters! 🫶**

## Contributing
Thanks in advance for helping to develop the Barter ecosystem! Please do not hesitate to get touch via the Discord [Chat] to discuss development,
new features, and the future roadmap.

### Licence
This project is licensed under the [MIT license].

[MIT license]: https://github.com/barter-rs/barter-rs/blob/develop/LICENSE

### Contribution License Agreement

Any contribution you intentionally submit for inclusion in Barter workspace crates shall be:
1. Licensed under MIT
2. Subject to all disclaimers and limitations of liability stated below
3. Provided without any additional terms or conditions
4. Submitted with the understanding that the educational-only purpose and risk warnings apply

By submitting a contribution, you certify that you have the right to do so under these terms.

## LEGAL DISCLAIMER AND LIMITATION OF LIABILITY

PLEASE READ THIS DISCLAIMER CAREFULLY BEFORE USING THE SOFTWARE. BY ACCESSING OR USING THE SOFTWARE, YOU ACKNOWLEDGE AND AGREE TO BE BOUND BY THE TERMS HEREIN.

1. EDUCATIONAL PURPOSE
   This software and related documentation ("Software") are provided solely for educational and research purposes. The Software is not intended, designed, tested, verified or certified for commercial deployment, live trading, or production use of any kind.

2. NO FINANCIAL ADVICE
   Nothing contained in the Software constitutes financial, investment, legal, or tax advice. No aspect of the Software should be relied upon for trading decisions or financial planning. Users are strongly advised to consult qualified professionals for investment guidance specific to their circumstances.

3. ASSUMPTION OF RISK
   Trading in financial markets, including but not limited to cryptocurrencies, securities, derivatives, and other financial instruments, carries substantial risk of loss. Users acknowledge that:
   a) They may lose their entire investment;
   b) Past performance does not indicate future results;
   c) Hypothetical or simulated performance results have inherent limitations and biases.

4. DISCLAIMER OF WARRANTIES
   THE SOFTWARE IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED. TO THE MAXIMUM EXTENT PERMITTED BY LAW, THE AUTHORS AND COPYRIGHT HOLDERS EXPRESSLY DISCLAIM ALL WARRANTIES, INCLUDING BUT NOT LIMITED TO:
   a) MERCHANTABILITY
   b) FITNESS FOR A PARTICULAR PURPOSE
   c) NON-INFRINGEMENT
   d) ACCURACY OR RELIABILITY OF RESULTS
   e) SYSTEM INTEGRATION
   f) QUIET ENJOYMENT

5. LIMITATION OF LIABILITY
   IN NO EVENT SHALL THE AUTHORS, COPYRIGHT HOLDERS, CONTRIBUTORS, OR ANY AFFILIATED PARTIES BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING BUT NOT LIMITED TO PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES, LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

6. REGULATORY COMPLIANCE
   The Software is not registered with, endorsed by, or approved by any financial regulatory authority. Users are solely responsible for:
   a) Determining whether their use complies with applicable laws and regulations
   b) Obtaining any required licenses, permits, or registrations
   c) Meeting any regulatory obligations in their jurisdiction

7. INDEMNIFICATION
   Users agree to indemnify, defend, and hold harmless the authors, copyright holders, and any affiliated parties from and against any claims, liabilities, damages, losses, and expenses arising from their use of the Software.

8. ACKNOWLEDGMENT
   BY USING THE SOFTWARE, USERS ACKNOWLEDGE THAT THEY HAVE READ THIS DISCLAIMER, UNDERSTOOD IT, AND AGREE TO BE BOUND BY ITS TERMS AND CONDITIONS.

THE ABOVE LIMITATIONS MAY NOT APPLY IN JURISDICTIONS THAT DO NOT ALLOW THE EXCLUSION OF CERTAIN WARRANTIES OR LIMITATIONS OF LIABILITY.