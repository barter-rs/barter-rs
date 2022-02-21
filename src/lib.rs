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
//! [`Barter`]: https://gitlab.com/open-source-keir/financial-modelling/trading/barter-rs
//! [`Barter-Data`]: https://crates.io/crates/barter-data
//! [`Readme`]: https://crates.io/crates/barter
//!
//! ## Getting Started
//! ### Data Handler
//! ```
//! use barter::data::handler::{Continuation, Continuer, MarketGenerator};
//! use barter::data::handler::historical::{HistoricalCandleHandler, HistoricalDataLego};
//! use barter_data::test_util;
//!
//! let lego = HistoricalDataLego {
//!     exchange: "binance",
//!     symbol: "btc_usdt".to_string(),
//!     candles: vec![test_util::candle(), test_util::candle()].into_iter(),
//! };
//!
//! let mut data = HistoricalCandleHandler::new(lego);
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
//! use barter::data::MarketEvent;
//! use barter::test_util;
//!
//! let config = StrategyConfig {
//!     rsi_period: 14,
//! };
//!
//! let mut strategy = RSIStrategy::new(config);
//!
//! let market_event = test_util::market_event();
//!
//! let signal_event = strategy.generate_signal(&market_event);
//! ```
//!
//! ### Portfolio
//! ```
//! use barter::portfolio::{MarketUpdater, OrderGenerator, FillUpdater};
//! use barter::portfolio::allocator::DefaultAllocator;
//! use barter::portfolio::risk::DefaultRisk;
//! use barter::portfolio::repository::redis::RedisRepository;
//! use barter::event::Event;
//! use barter::Market;
//! use barter::portfolio::OrderEvent;
//! use barter::portfolio::portfolio::{PortfolioLego, MetaPortfolio};
//! use barter::portfolio::repository::redis::Config as RepositoryConfig;
//! use barter::portfolio::repository::in_memory::InMemoryRepository;
//! use barter::statistic::summary::pnl::PnLReturnSummary;
//! use barter::statistic::summary::trading::{Config as StatisticConfig, TradingSummary};
//! use barter::test_util;
//! use std::marker::PhantomData;
//! use uuid::Uuid;
//!
//! let components = PortfolioLego {
//!     engine_id: Uuid::new_v4(),
//!     markets: vec![Market {exchange: "binance",symbol: "btc_usdt".to_string()}],
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
//! use barter::execution::simulated::{Config as ExecutionConfig, SimulatedExecution};
//! use barter::portfolio::OrderEvent;
//! use barter::execution::Fees;
//! use barter::execution::ExecutionClient;
//! use barter::test_util;
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
//! use barter::statistic::summary::trading::{Config as StatisticConfig, TradingSummary};
//! use barter::portfolio::position::Position;
//! use barter::statistic::summary::{Initialiser, PositionSummariser, TablePrinter};
//! use barter::test_util;
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
//! trading_summary.print();
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

use serde::Serialize;

/// Communicates a str is a unique identifier for an Exchange (eg/ "binance")
pub type ExchangeId = &'static str;

/// Communicates a String is a unique identifier for a pair symbol (eg/ "btc_usd")
pub type SymbolId = String;

/// Communicates a String represents a unique market identifier (eg/ "market_binance-btc_usdt").
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
            symbol: symbol.to_lowercase(),
        }
    }

    /// Returns the [`MarketId`] unique identifier associated with this [`Market`] by utilising
    /// [`determine_market_id`] (eg/ "market_binance-btc_usdt").
    pub fn market_id(&self) -> MarketId {
        determine_market_id(self.exchange, &self.symbol)
    }
}

/// Returns the unique identifier for a given market, where a 'market' is a unique
/// exchange-symbol combination (eg/ "market_binance-btc_usdt").
pub fn determine_market_id(exchange: &str, symbol: &str) -> MarketId {
    format!("market_{}_{}", exchange, symbol)
}

pub mod test_util {
    use crate::data::{MarketEvent, MarketMeta};
    use crate::strategy::{Decision, SignalEvent};
    use crate::portfolio::{OrderEvent, OrderType};
    use crate::portfolio::position::Position;
    use crate::execution::{Fees, FillEvent};
    use barter_data::model::MarketData;
    use barter_data::test_util;
    use chrono::Utc;
    use uuid::Uuid;

    pub fn market_event() -> MarketEvent {
        MarketEvent {
            event_type: MarketEvent::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: "binance",
            symbol: String::from("eth_usdt"),
            data: MarketData::Candle(test_util::candle()),
        }
    }

    pub fn signal_event() -> SignalEvent {
        SignalEvent {
            event_type: SignalEvent::ORGANIC_SIGNAL,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: "binance",
            symbol: String::from("eth_usdt"),
            market_meta: MarketMeta::default(),
            signals: Default::default(),
        }
    }

    pub fn order_event() -> OrderEvent {
        OrderEvent {
            event_type: OrderEvent::ORGANIC_ORDER,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: "binance",
            symbol: String::from("eth_usdt"),
            market_meta: MarketMeta::default(),
            decision: Decision::default(),
            quantity: 1.0,
            order_type: OrderType::default(),
        }
    }

    pub fn fill_event() -> FillEvent {
        FillEvent {
            event_type: FillEvent::EVENT_TYPE,
            trace_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            exchange: "binance",
            symbol: String::from("eth_usdt"),
            market_meta: Default::default(),
            decision: Decision::default(),
            quantity: 1.0,
            fill_value_gross: 100.0,
            fees: Fees::default(),
        }
    }

    pub fn position() -> Position {
        Position {
            position_id: "engine_id_trader_{}_{}_position".to_owned(),
            exchange: "binance".to_owned(),
            symbol: "eth_usdt".to_owned(),
            meta: Default::default(),
            direction: Default::default(),
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