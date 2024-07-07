//! # Barter
//! [`Barter`] is an open-source Rust framework for building **event-driven live-trading & backtesting systems**.
//! Algorithmic trade with the peace of mind that comes from knowing your strategies have been
//! backtested with a near-identical trading Engine.
//! It is:
//! * **Fast**: Barter provides a multi-threaded trading Engine framework built in high-performance Rust (in-rust-we-trust).
//! * **Easy**: Barter provides a modularised data architecture that focuses on simplicity.
//! * **Customisable**: A set of traits define how every Barter component communicates, providing a highly extensible
//! framework for trading.
//!
//! See [`Readme`].
//!
//! ## Overview
//! Barter is an open-source Rust framework for building **event-driven live-trading & backtesting systems**. It provides
//! a high-performance, easy to customise, trading Engine that enables backtesting strategies on a near-identical system
//! to live trading. The Engine can be **controlled by issuing Commands** over the Engine's command_tx. Similarly,
//! the **Engine's Events can be listened to using the event_rx** (useful for event-sourcing). At a high level,
//! it provides several de-coupled components that interact via a set of traits:

//! * **Data**: Continuer & MarketGenerator traits govern the generation of a MarketEvents data feed that acts as the system
//! heartbeat. For example, a LiveCandleHandler implementation is provided utilising [`Barter-Data`]'s WebSocket functionality to
//! provide a live market Candle data feed to the system.
//! * **Strategy**: The SignalGenerator trait governs potential generation of SignalEvents after analysing incoming
//! MarketEvents. SignalEvents are advisory signals sent to the Portfolio for analysis.
//! * **Portfolio**: MarketUpdater, OrderGenerator, and FillUpdater govern global state Portfolio implementations. A
//! Portfolio may generate OrderEvents after receiving advisory SignalEvents from a Strategy. The Portfolio's state
//! updates after receiving MarketEvents and FillEvents.
//! * **Execution**: The FillGenerator trait governs the generation of FillEvents after receiving OrderEvents from the
//! Portfolio. For example, a SimulatedExecution handler implementation is provided for simulating any exchange execution
//! behaviour required in dry-trading or backtesting runs.
//! * **Statistic**: Provides metrics such as Sharpe Ratio, Calmar Ratio, and Max Drawdown to analyse trading session
//! performance. One-pass dispersion algorithms analyse each closed Position and efficiently calculates a trading summary.
//! * **Trader**: Capable of trading a single market pair using a customisable selection of it's own Data, Strategy &
//! Execution instances, as well as shared access to a global Portfolio.
//! * **Engine**: Multi-threaded trading Engine capable of trading with an arbitrary number of Trader market pairs. Each
//! contained Trader instance operates on its own thread.
//!
//! [`Barter`]: https://github.com/barter-rs/barter-rs
//! [`Barter-Data`]: https://crates.io/crates/barter-data
//! [`Readme`]: https://crates.io/crates/barter
//!
//! ## Getting Started
//! ### Data Handler
//! ```
//!
//!
//! use barter::{data::{Feed, historical, MarketGenerator}, test_util};
//! use barter_integration::model::Side;
//!
//! let mut data = historical::MarketFeed::new([test_util::market_event_trade(Side::Buy)].into_iter());
//!
//! loop {
//!     let market_event = match data.next() {
//!         Feed::Next(market_event) => market_event,
//!         Feed::Finished => break,
//!         Feed::Unhealthy => continue,
//!     };
//! }
//! ```
//!
//! ### Strategy
//! ```
//! use barter::{
//!     strategy::{SignalGenerator, example::{Config as StrategyConfig, RSIStrategy}},
//!     test_util,
//! };
//! use barter_integration::model::Side;
//!
//! let config = StrategyConfig {
//!     rsi_period: 14,
//! };
//!
//! let mut strategy = RSIStrategy::new(config);
//!
//! let market_event = test_util::market_event_trade(Side::Buy);
//!
//! let signal_event = strategy.generate_signal(&market_event);
//! ```
//!
//! ### Portfolio
//! ```
//! use barter::{
//!     portfolio::{
//!         MarketUpdater, OrderGenerator, FillUpdater,
//!         portfolio::{PortfolioLego, MetaPortfolio},
//!         repository::in_memory::InMemoryRepository,
//!         allocator::DefaultAllocator,
//!         risk::DefaultRisk,
//!     },
//!     statistic::summary::{
//!         pnl::PnLReturnSummary,
//!         trading::{Config as StatisticConfig, TradingSummary},
//!     },
//!     event::Event,
//!     test_util,
//! };
//! use barter_integration::model::{Market, instrument::kind::InstrumentKind};
//! use std::marker::PhantomData;
//! use uuid::Uuid;
//!
//! let components = PortfolioLego {
//!     engine_id: Uuid::new_v4(),
//!     markets: vec![Market::new("binance", ("btc", "usdt", InstrumentKind::Spot))],
//!     repository: InMemoryRepository::new(),
//!     allocator: DefaultAllocator{ default_order_value: 100.0 },
//!     risk: DefaultRisk{},
//!     starting_cash: 10000.0,
//!     statistic_config: StatisticConfig {
//!         starting_equity: 10000.0 ,
//!         trading_days_per_year: 365,
//!         risk_free_return: 0.0
//!     },
//!     _statistic_marker: PhantomData::<TradingSummary>::default()
//! };
//!
//! let mut portfolio = MetaPortfolio::init(components).unwrap();
//!
//! let some_event = Event::OrderNew(test_util::order_event());
//!
//! match some_event {
//!     Event::Market(market) => {
//!         portfolio.update_from_market(&market);
//!     }
//!     Event::Signal(signal) => {
//!         portfolio.generate_order(&signal);
//!     }
//!     Event::SignalForceExit(signal) => {
//!         portfolio.generate_exit_order(signal);
//!     }
//!     Event::Fill(fill) => {
//!         portfolio.update_from_fill(&fill);
//!     }
//!     _ => {}
//! }
//! ```
//!
//! ### Execution
//! ```
//! use barter::{
//!     test_util,
//!     portfolio::OrderEvent,
//!     execution::{
//!         simulated::{Config as ExecutionConfig, SimulatedExecution},
//!         Fees, ExecutionClient,
//!     }
//! };
//!
//! let config = ExecutionConfig {
//!     simulated_fees_pct: Fees {
//!         exchange: 0.1,
//!         slippage: 0.05, // Simulated slippage modelled as a Fee
//!         network: 0.0,
//!     }
//! };
//!
//! let mut execution = SimulatedExecution::new(config);
//!
//! let order_event = test_util::order_event();
//!
//! let fill_event = execution.generate_fill(&order_event);
//! ```
//!
//! ### Statistic
//! ```
//! use barter::{
//!     test_util,
//!     portfolio::position::Position,
//!     statistic::summary::{
//!         trading::{Config as StatisticConfig, TradingSummary},
//!         Initialiser, PositionSummariser, TableBuilder
//!     }
//! };
//!
//! // Do some automated trading with barter components that generates a vector of closed Positions
//! let positions = vec![test_util::position(), test_util::position()];
//!
//! let config = StatisticConfig {
//!     starting_equity: 10000.0,
//!     trading_days_per_year: 253,
//!     risk_free_return: 0.5,
//! };
//!
//! let mut trading_summary = TradingSummary::init(config);
//!
//! trading_summary.generate_summary(&positions);
//!
//! trading_summary
//!     .table("Total")
//!     .printstd();
//! ```
//!
//! ### Engine & Traders
//! [See Readme Engine Example](https://crates.io/crates/barter#example)

#![warn(
    unused,
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]
#![allow(clippy::type_complexity)]
#![allow(clippy::module_inception)]

/// Defines a MarketEvent, and provides the Continuer and MarketGenerator traits for
/// handling the generation of them. Contains implementations such as the (tick-by_tick)
/// LiveTradeHandler, and HistoricalCandleHandler that generates a market feed and acts as the
/// system heartbeat.
pub mod data;

/// Defines a SignalEvent and SignalForceExit, as well as the SignalGenerator trait for handling the
/// generation of them. Contains an example RSIStrategy implementation that analyses a MarketEvent
/// and may generate a new advisory SignalEvent to be analysed by the Portfolio OrderGenerator.
pub mod strategy;

/// Defines useful data structures such as an OrderEvent and Position. The Portfolio must
/// interact with MarketEvents, SignalEvents, OrderEvents, and FillEvents. The useful traits
/// MarketUpdater, OrderGenerator, & FillUpdater are provided that define the interactions
/// with these events. Contains a MetaPortfolio implementation that persists state in a
/// generic Repository. This also contains example implementations of an OrderAllocator &
/// OrderEvaluator, which help the Portfolio make decisions on whether to generate OrderEvents and
/// of what size.
pub mod portfolio;

/// Defines a FillEvent, and provides a useful trait FillGenerator for handling the generation
/// of them. Contains an example SimulatedExecution implementation that simulates live broker
/// execution.
pub mod execution;

/// Defines an Event enum that contains variants that are vital to the trading event loop
/// (eg/ MarketEvent). Other variants communicate work done by the system (eg/ FillEvent), as well
/// as changes in system state (eg/ PositionUpdate).
pub mod event;

/// Defines various iterative statistical methods that can be used to calculate trading performance
/// metrics in one-pass. A trading performance summary implementation has been provided containing
/// several key metrics such as Sharpe Ratio, Calmar Ratio, and Max Drawdown.
pub mod statistic;

/// Multi-threaded trading Engine capable of trading with an arbitrary number market pairs. Contains
/// a Trader for each Market pair that consists of it's own Data, Strategy &
/// Execution components, as well as shared access to a global Portfolio.
pub mod engine;

#[macro_use]
extern crate prettytable;

pub mod test_util {
    use crate::{
        data::MarketMeta,
        execution::{Fees, FillEvent},
        portfolio::{position::Position, OrderEvent, OrderType},
        strategy::{Decision, Signal},
    };
    use barter_data::{
        event::{DataKind, MarketEvent},
        exchange::ExchangeId,
        subscription::{candle::Candle, trade::PublicTrade},
    };
    use barter_integration::model::{
        instrument::{kind::InstrumentKind, Instrument},
        Exchange, Side,
    };
    use chrono::Utc;
    use std::ops::Add;

    /// Build a [`MarketEvent`] of [`DataKind::PublicTrade`](DataKind) with the provided [`Side`].
    pub fn market_event_trade(side: Side) -> MarketEvent<DataKind> {
        MarketEvent {
            exchange_time: Utc::now(),
            received_time: Utc::now(),
            exchange: Exchange::from(ExchangeId::BinanceSpot),
            instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)),
            kind: DataKind::Trade(PublicTrade {
                id: "trade_id".to_string(),
                price: 1000.0,
                amount: 1.0,
                side,
            }),
        }
    }

    /// Build a [`MarketEvent`] of [`DataKind::Candle`](DataKind).
    pub fn market_event_candle() -> MarketEvent<DataKind> {
        let now = Utc::now();
        MarketEvent {
            exchange_time: now,
            received_time: now.add(chrono::Duration::milliseconds(200)),
            exchange: Exchange::from(ExchangeId::BinanceSpot),
            instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)),
            kind: DataKind::Candle(Candle {
                close_time: now,
                open: 960.0,
                high: 1100.0,
                low: 950.0,
                close: 1000.0,
                volume: 100000.0,
                trade_count: 1000,
            }),
        }
    }

    /// Build a [`Signal`].
    pub fn signal() -> Signal {
        Signal {
            time: Utc::now(),
            exchange: Exchange::from("binance"),
            instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)),
            signals: Default::default(),
            market_meta: Default::default(),
        }
    }

    /// Build an [`OrderEvent`] to buy 1.0 contract.
    pub fn order_event() -> OrderEvent {
        OrderEvent {
            time: Utc::now(),
            exchange: Exchange::from("binance"),
            instrument: Instrument::from(("eth", "usdt", InstrumentKind::Spot)),
            market_meta: MarketMeta::default(),
            decision: Decision::default(),
            quantity: 1.0,
            order_type: OrderType::default(),
        }
    }

    /// Build a [`FillEvent`] for a single bought contract.
    pub fn fill_event() -> FillEvent {
        FillEvent {
            time: Utc::now(),
            exchange: Exchange::from("binance"),
            instrument: Instrument::from(("eth", "usdt", InstrumentKind::Spot)),
            market_meta: Default::default(),
            decision: Decision::default(),
            quantity: 1.0,
            fill_value_gross: 100.0,
            fees: Fees::default(),
        }
    }

    /// Build a [`Position`].
    pub fn position() -> Position {
        Position {
            position_id: "engine_id_trader_{}_{}_position".to_owned(),
            exchange: Exchange::from("binance"),
            instrument: Instrument::from(("eth", "usdt", InstrumentKind::Spot)),
            meta: Default::default(),
            side: Side::Buy,
            quantity: 1.0,
            enter_fees: Default::default(),
            enter_fees_total: 0.0,
            enter_avg_price_gross: 100.0,
            enter_value_gross: 100.0,
            exit_fees: Default::default(),
            exit_fees_total: 0.0,
            exit_avg_price_gross: 0.0,
            exit_value_gross: 0.0,
            current_symbol_price: 100.0,
            current_value_gross: 100.0,
            unrealised_profit_loss: 0.0,
            realised_profit_loss: 0.0,
        }
    }
}
