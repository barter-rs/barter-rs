//! [Barter] is an open-source Rust library containing **high-performance**, **modular** trading
//! engine & backtesting **components**.
//!
//!
//! # Overview
//! The **main components** are **Data**, **Strategy**, **Portfolio**, **Execution** & **Statistic**.
//!
//! Each components is stand-alone & de-coupled. Their behaviour is captured
//! in a set of useful traits that define how each component responds to external events.
//!
//! The **Data**, **Strategy** & **Execution components** are designed to be used by **one trading
//! pair only** (eg/ ETH-USD on Binance). In order to **trade multiple pairs** on multiple exchanges,
//! **many combinations** of these three components should be constructed and **run concurrently**.
//!
//! The **Portfolio** component is designed in order to **manage infinite trading pairs simultaneously**,
//! creating a **centralised state machine** for managing trades across different asset classes,
//! exchanges, and trading pairs. It's likely you might want to build an API on top of a Portfolio
//! and run it as a micro-service. On the other hand, it can also be run in conjunction with the other
//! components to provide a Portfolio for an individual trading pair.
//!
//! The **Statistic** component contains metrics used to analyse trading session performance.
//! One-pass dispersion algorithms analyse each closed Position and efficiently update
//! a PnL Return Summary. This summary, in conjunction with the closed Position, is used to calculate
//! key metrics such as Sharpe Ratio, Calmar Ratio and Max Drawdown. All metrics can be updated on
//! the fly and used by the Portfolio's allocation & risk management.
//!
//! **Example high-level data architecture using these components in this** [README].
//!
//! # Getting Started
//! ## Data Handler
//! ```
//!
//! use barter::data::handler::{HistoricDataHandler, MarketGenerator, Continuer};
//! use barter::data::market::Bar;
//!
//! // Or create with HistoricDataHandler::new(...) and pass a symbol data file to be parsed!
//! let mut data = HistoricDataHandler::builder()
//!     .symbol("ETH-USD".to_string())
//!     .exchange("BINANCE".to_string())
//!     .all_symbol_data(vec![Bar::default()].into_iter())
//!     .build().unwrap();
//!
//! loop {
//!     if data.should_continue() {
//!         let market_event = data.generate_market().unwrap();
//!     } else {
//!         // Pass closed Positions to statistics module for performance analysis
//!         break;
//!     }
//!
//! }
//! ```
//!
//! ## Strategy
//! ```
//! use barter::strategy::strategy::{Config as StrategyConfig, RSIStrategy, SignalGenerator};
//! use barter::data::market::MarketEvent;
//!
//!
//! let config = StrategyConfig {
//!     rsi_period: 14,
//! };
//!
//! let mut strategy = RSIStrategy::new(&config);
//!
//! let market_event = MarketEvent::default();
//!
//! let signal_event = strategy.generate_signal(&market_event);
//! ```
//!
//! ## Portfolio
//! ```
//! use barter::portfolio::portfolio::{Components, MetaPortfolio, MarketUpdater, OrderGenerator, FillUpdater};
//! use barter::portfolio::allocator::DefaultAllocator;
//! use barter::portfolio::risk::DefaultRisk;
//! use barter::portfolio::repository::redis::RedisRepository;
//! use barter::event::Event;
//! use barter::portfolio::order::OrderEvent;
//! use barter::portfolio::repository::redis::Config as RepositoryConfig;
//! use barter::portfolio::repository::in_memory::InMemoryRepository;
//!
//! let components = Components {
//!     allocator: DefaultAllocator{ default_order_value: 100.0 },
//!     risk: DefaultRisk{},
//!     starting_cash: 10000.0,
//! };
//!
//! let repository = InMemoryRepository::new();
//!
//! let mut portfolio = MetaPortfolio::new(components, repository);
//!
//! let some_event = Event::Order(OrderEvent::default());
//!
//! match some_event {
//!     Event::Market(market) => {
//!         portfolio.update_from_market(&market);
//!     }
//!     Event::Signal(signal) => {
//!         portfolio.generate_order(&signal);
//!     }
//!     Event::Order(order) => {
//!         // Not relevant
//!     }
//!     Event::Fill(fill) => {
//!         portfolio.update_from_fill(&fill);
//!     }
//! }
//! ```
//!
//! ## Execution
//! ```
//! use barter::execution::handler::{Config as ExecutionConfig, SimulatedExecution, FillGenerator};
//! use barter::portfolio::order::OrderEvent;
//! use barter::execution::fill::Fees;
//!
//! let config = ExecutionConfig {
//!     simulated_fees_pct: Fees {
//!         exchange: 0.1,
//!         slippage: 0.05, // Simulated slippage modelled as a Fee
//!         network: 0.0,
//!     }
//! };
//!
//! let mut execution = SimulatedExecution::new(&config);
//!
//! let order_event = OrderEvent::default();
//!
//! let fill_event = execution.generate_fill(&order_event);
//! ```
//!
//! ## Statistic
//! ```
//! use barter::statistic::summary::trading::{Config as StatisticConfig, TradingSummary, PositionSummariser, TablePrinter};
//! use barter::portfolio::position::Position;
//!
//! // Do some automated trading with barter components that generates a vector of closed Positions
//! let positions = vec![Position::default(), Position::default()];
//!
//! let config = StatisticConfig {
//!     starting_equity: 10000.0,
//!     trading_days_per_year: 253,
//!     risk_free_return: 0.5,
//! };
//!
//! let mut trading_summary = TradingSummary::new(&config);
//!
//! trading_summary.generate_summary(&positions);
//!
//! trading_summary.print();
//! ```
//!
//! # Examples
//!
//! [Barter]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-rs
//! [README]: https://crates.io/crates/barter

/// Defines a MarketEvent, and provides the useful traits of Continuer and MarketGenerator for
/// handling the generation of them. Contains implementations such as the HistoricDataHandler that
/// simulates a live market feed and acts as the systems heartbeat.
pub mod data;

/// Defines a SignalEvent, and provides the SignalGenerator trait for handling the generation of
/// them. Contains an example RSIStrategy implementation that analyses a MarketEvent and may
/// generate a new SignalEvent, an advisory signal for a Portfolio OrderGenerator to analyse.
pub mod strategy;

/// Defines useful data structures such as an OrderEvent and Position. The Portfolio must
/// interact with MarketEvents, SignalEvents, OrderEvents, and FillEvents. The useful traits
/// MarketUpdater, OrderGenerator, & FillUpdater are provided that define the interactions
/// with these events. Contains a MetaPortfolio implementation that persists state in a
/// RedisRepository. This also contains example implementations of a OrderAllocator &
/// OrderEvaluator, and help the Portfolio make decisions on whether to generate OrderEvents and
/// of what size.
pub mod portfolio;

/// Defines a FillEvent, and provides a useful trait FillGenerator for handling the generation
/// of them. Contains a SimulatedExecution implementation that simulates a live broker execution
/// for the system.
pub mod execution;

/// Defines an Event enum that could be a MarketEvent, SignalEvent, OrderEvent or
/// FillEvent.
pub mod event;

/// Defines various iterative statistical methods that can be used to calculate trading performance
/// metrics in one-pass. A trading performance summary implementation has been provided containing
/// several key metrics such as Sharpe Ratio, Calmar Ratio, CAGR, and Max Drawdown.
pub mod statistic;

#[macro_use]
extern crate prettytable;
