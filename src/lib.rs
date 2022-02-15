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
//! to live trading. At a high level, it provides several de-coupled components that interact via a set of traits:

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
//! [`Barter`]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-rs
//! [`Barter-Data`]: https://crates.io/crates/barter-data
//! [`Readme`]: https://crates.io/crates/barter
//!
//! ## Getting Started
//! ### Data Handler
//! ```
//! use barter_data::model::Candle;
//! use barter::data::handler::{Continuation, Continuer, MarketGenerator};
//! use barter::data::handler::historic::{HistoricCandleHandler, HistoricDataLego};
//!
//! let lego = HistoricDataLego {
//!     exchange: "Binance",
//!     symbol: "btcusdt".to_string(),
//!     candles: vec![Candle::default()].into_iter(),
//! };
//!
//! let mut data = HistoricCandleHandler::new(lego);
//!
//! loop {
//!     let market_event = match data.can_continue() {
//!         Continuation::Continue => {
//!             match data.generate_market() {
//!                 Some(market_event) => market_event,
//!                 None => continue,
//!             }
//!
//!         }
//!         Continuation::Stop => {
//!             // Pass closed Positions to Statistic module for performance analysis
//!             break;
//!         }
//!     };
//! }
//!
//! ```
//!
//! ### Strategy
//! ```
//! use barter::strategy::SignalGenerator;
//! use barter::strategy::strategy::{Config as StrategyConfig, RSIStrategy};
//! use barter::data::market::MarketEvent;
//!
//! let config = StrategyConfig {
//!     rsi_period: 14,
//! };
//!
//! let mut strategy = RSIStrategy::new(config);
//!
//! let market_event = MarketEvent::default();
//!
//! let signal_event = strategy.generate_signal(&market_event);
//! ```
//!
//! ### Portfolio
//! ```
//! use std::marker::PhantomData;
//! use barter::portfolio::{MarketUpdater, OrderGenerator, FillUpdater};
//! use barter::portfolio::allocator::DefaultAllocator;
//! use barter::portfolio::risk::DefaultRisk;
//! use barter::portfolio::repository::redis::RedisRepository;
//! use barter::event::Event;
//! use barter::Market;
//! use barter::portfolio::order::OrderEvent;
//! use barter::portfolio::portfolio::{PortfolioLego, MetaPortfolio};
//! use barter::portfolio::repository::redis::Config as RepositoryConfig;
//! use barter::portfolio::repository::in_memory::InMemoryRepository;
//! use barter::statistic::summary::pnl::PnLReturnSummary;
//! use barter::statistic::summary::trading::{Config as StatisticConfig, TradingSummary};
//!
//! let components = PortfolioLego {
//!     engine_id: Default::default(),
//!     markets: vec![Market {exchange: "binance",symbol: "btcusdt".to_string()}],
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
//! let some_event = Event::OrderNew(OrderEvent::default());
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
//! use barter::execution::handler::{Config as ExecutionConfig, SimulatedExecution};
//! use barter::portfolio::order::OrderEvent;
//! use barter::execution::fill::Fees;
//! use barter::execution::FillGenerator;
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
//! let order_event = OrderEvent::default();
//!
//! let fill_event = execution.generate_fill(&order_event);
//! ```
//!
//! ### Statistic
//! ```
//! use barter::statistic::summary::trading::{Config as StatisticConfig, TradingSummary};
//! use barter::portfolio::position::Position;
//! use barter::statistic::summary::{PositionSummariser, TablePrinter};
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
//! let mut trading_summary = TradingSummary::new(config);
//!
//! trading_summary.generate_summary(&positions);
//!
//! trading_summary.print();
//! ```
//!
//! ### Engine & Traders
//! [See Readme Engine Example](https://crates.io/crates/barter#example)

#![warn(
// missing_debug_implementations,
missing_copy_implementations,
// rust_2018_idioms,
// missing_docs
)]

/// Defines a MarketEvent, and provides the useful traits of Continuer and MarketGenerator for
/// handling the generation of them. Contains implementations such as the LiveCandleHandler that
/// generates a live market feed and acts as the system heartbeat.
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

/// Defines an Event enum that contains variants that are vital to the trading event loop
/// (eg/ MarketEvent). Other variants communicate work done by the system (eg/ FillEvent), as well
/// as changes in system state (eg/ PositionUpdate).
pub mod event;

/// Defines various iterative statistical methods that can be used to calculate trading performance
/// metrics in one-pass. A trading performance summary implementation has been provided containing
/// several key metrics such as Sharpe Ratio, Calmar Ratio, CAGR, and Max Drawdown.
pub mod statistic;

/// Multi-threaded trading Engine capable of trading with an arbitrary number market pairs. Contains
/// a Trader instance for each market pair that consists of it's own Data, Strategy &
/// Execution instances, as well as shared access to a global Portfolio.
pub mod engine;

#[macro_use]
extern crate prettytable;

use serde::Serialize;

/// Communicates a str is a unique identifier for an Exchange (eg/ "binance")
pub type ExchangeId = &'static str;

/// Communicates a String is a unique identifier for a pair symbol (eg/ "BTC_USDT")
pub type SymbolId = String;

/// Communicates a String represents a unique market identifier (eg/ "market_binance-USDT_BTC").
pub type MarketId = String;

/// Represents a unique combination of an [`ExchangeId`] & a [`SymbolId`]. Each [`Trader`] barters
/// on one [`Market`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct Market {
    pub exchange: ExchangeId,
    pub symbol: SymbolId,
}

impl Market {
    /// Constructs a new [`Market`] using the provided [`ExchangeId`] & [`SymbolId`].
    pub fn new(exchange: ExchangeId, symbol: SymbolId) -> Self {
        Self {
            exchange,
            symbol: symbol.to_lowercase()
        }
    }

    /// Returns the [`MarketId`] unique identifier associated with this [`Market`] by utilising
    /// [`determine_market_id`] (eg/ "market_binance-USDT_BTC").
    pub fn market_id(&self) -> MarketId {
        determine_market_id(self.exchange, &self.symbol)
    }
}

/// Returns the unique identifier for a given market, where a 'market' is a unique
/// exchange-symbol combination (eg/ "market_binance-USDT_BTC").
pub fn determine_market_id(exchange: &str, symbol: &String) -> MarketId {
    format!("market_{}_{}", exchange, symbol)
}