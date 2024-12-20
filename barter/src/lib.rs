#![forbid(unsafe_code)]
#![warn(
    unused,
    clippy::cognitive_complexity,
    unused_crate_dependencies,
    unused_extern_crates,
    clippy::unused_self,
    clippy::useless_let_if_seq,
    missing_debug_implementations,
    rust_2018_idioms,
    rust_2024_compatibility
)]
#![allow(clippy::type_complexity, clippy::too_many_arguments, type_alias_bounds)]

//! # Barter
//! [`Barter`] is an open-source Rust framework for building **event-driven live-trading & back-testing systems**.
//! Algorithmic trade with the peace of mind that comes from knowing your strategies have been
//! backtested with a near-identical trading Engine.
//! It is:
//! * **Fast**: Barter provides a multithreaded trading Engine framework built in high-performance Rust (in-rust-we-trust).
//! * **Easy**: Barter provides a modularised data architecture that focuses on simplicity.
//! * **Customisable**: A set of traits define how every Barter component communicates, providing a highly extensible
//!   framework for trading.
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
//!   heartbeat. For example, a LiveCandleHandler implementation is provided utilising [`Barter-Data`]'s WebSocket functionality to
//!   provide a live market Candle data feed to the system.
//! * **Strategy**: The SignalGenerator trait governs potential generation of SignalEvents after analysing incoming
//!   MarketEvents. SignalEvents are advisory signals sent to the Portfolio for analysis.
//! * **Portfolio**: MarketUpdater, OrderGenerator, and FillUpdater govern global state Portfolio implementations. A
//!   Portfolio may generate OrderEvents after receiving advisory SignalEvents from a Strategy. The Portfolio's state
//!   updates after receiving MarketEvents and FillEvents.
//! * **Execution**: The FillGenerator trait governs the generation of FillEvents after receiving OrderEvents from the
//!   Portfolio. For example, a SimulatedExecution handler implementation is provided for simulating any exchange execution
//!   behaviour required in dry-trading or backtesting runs.
//! * **Statistic**: Provides metrics such as Sharpe Ratio, Calmar Ratio, and Max Drawdown to analyse trading session
//!   performance. One-pass dispersion algorithms analyse each closed Position and efficiently calculates a trading summary.
//! * **Trader**: Capable of trading a single market pair using a customisable selection of its own Data, Strategy &
//!   Execution instances, as well as shared access to a global Portfolio.
//! * **Engine**: Multi-threaded trading Engine capable of trading with an arbitrary number of Trader market pairs. Each
//!   contained Trader instance operates on its own thread.
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
//! use barter_instrument::Side;
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
//! use barter_instrument::Side;
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
//! use std::marker::PhantomData;
//! use uuid::Uuid;
//! use barter_instrument::execution::ExchangeId;
//! use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
//! use barter_instrument::market::Market;
//!
//! let components = PortfolioLego {
//!     engine_id: Uuid::new_v4(),
//!     markets: vec![Market::new(ExchangeId::BinanceSpot, ("btc", "usdt", MarketDataInstrumentKind::Spot))],
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
//!         execution: 0.1,
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

use crate::{
    engine::{command::Command, state::trading::TradingState},
    execution::AccountStreamEvent,
};
use barter_data::{event::MarketEvent, streams::consumer::MarketStreamEvent};
use barter_execution::AccountEvent;
use barter_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use chrono::{DateTime, Utc};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};

pub mod engine;
pub mod error;
pub mod execution;
pub mod risk;
pub mod statistic;
pub mod strategy;

// Todo: Must: Final Requirements
//  - Comprehensive rust docs & check output
//  - Comprehensive rust examples
//  - Comprehensive readme.md for each crate & workspace

// Todo: Spike: Keys
//  - Try to remove generic keys and enforce indexes
//  - See if any state management implementations can be removed from EngineState in favour of
//    smaller components. eg/ impl TradingStateManager for EngineState isn't really appropriate for
//    testing -> could impl for TradingState itself...

// Todo: MarketData:
// Todo: add utils for creating Instruments from basic base,quote,kind,exchange, etc.

// Todo: Must: Docs:
//  - engine/state/mod.rs docs (the rest are done).
//  - Whole engine/action module docs.
//  - Whole engine/audit module docs (or wherever it may be moved to).

// Todo: Must: General:
//  - Back-test utilities via Audit route w/ interactive mode
//    (backward would require Vec<State> to be created on .next()) (add compression using file system)
//  - Ensure Audit pathway doesn't duplicate Logs
//    '--> see claude convo with "module layer" etc.

// Todo: Must: Engine:
//   - Handle re-connections in ConnectivityStates with acceptable performance.

// Todo: Must: Market Data
//  - Ensure utils exist for creating of MarketDataStreams for historic feeds

// Todo: Must: Execution
//   - Full MockExecution

pub type FnvIndexMap<K, V> = indexmap::IndexMap<K, V, fnv::FnvBuildHasher>;
pub type FnvIndexSet<T> = indexmap::IndexSet<T, fnv::FnvBuildHasher>;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Deserialize, Serialize, Constructor,
)]
pub struct Timed<T> {
    value: T,
    time: DateTime<Utc>,
}

pub type IndexedEngineEvent<MarketKind> =
    EngineEvent<MarketKind, ExchangeIndex, AssetIndex, InstrumentIndex>;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From)]
pub enum EngineEvent<MarketKind, ExchangeKey, AssetKey, InstrumentKey> {
    Shutdown,
    Command(Command<ExchangeKey, AssetKey, InstrumentKey>),
    TradingStateUpdate(TradingState),
    Account(AccountStreamEvent<ExchangeKey, AssetKey, InstrumentKey>),
    Market(MarketStreamEvent<InstrumentKey, MarketKind>),
}

impl<MarketKind, ExchangeKey, AssetKey, InstrumentKey>
    From<AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>
    for EngineEvent<MarketKind, ExchangeKey, AssetKey, InstrumentKey>
{
    fn from(value: AccountEvent<ExchangeKey, AssetKey, InstrumentKey>) -> Self {
        Self::Account(AccountStreamEvent::Item(value))
    }
}

impl<MarketKind, ExchangeKey, AssetKey, InstrumentKey> From<MarketEvent<InstrumentKey, MarketKind>>
    for EngineEvent<MarketKind, ExchangeKey, AssetKey, InstrumentKey>
{
    fn from(value: MarketEvent<InstrumentKey, MarketKind>) -> Self {
        Self::Market(MarketStreamEvent::Item(value))
    }
}

#[cfg(test)]
pub mod test_utils {
    use crate::engine::state::asset::AssetState;
    use barter_execution::{
        balance::Balance,
        order::{OrderId, StrategyId},
        trade::{AssetFees, Trade, TradeId},
    };
    use barter_instrument::{
        asset::QuoteAsset, instrument::name::InstrumentNameInternal, test_utils::asset, Side,
    };
    use chrono::{DateTime, Days, Utc};

    pub fn f64_is_eq(actual: f64, expected: f64, epsilon: f64) -> bool {
        if actual.is_nan() && expected.is_nan() {
            true
        } else if actual.is_infinite() && expected.is_infinite() {
            actual.is_sign_positive() == expected.is_sign_positive()
        } else if actual.is_nan()
            || expected.is_nan()
            || actual.is_infinite()
            || expected.is_infinite()
        {
            false
        } else {
            (actual - expected).abs() < epsilon
        }
    }

    pub fn time_plus_days(base: DateTime<Utc>, plus: u64) -> DateTime<Utc> {
        base.checked_add_days(Days::new(plus)).unwrap()
    }

    pub fn trade(
        time_exchange: DateTime<Utc>,
        side: Side,
        price: f64,
        quantity: f64,
        fees: f64,
    ) -> Trade<QuoteAsset, InstrumentNameInternal> {
        Trade {
            id: TradeId::new("trade_id"),
            order_id: OrderId::new("order_id"),
            instrument: InstrumentNameInternal::new("instrument"),
            strategy: StrategyId::new("strategy"),
            time_exchange,
            side,
            price,
            quantity,
            fees: AssetFees {
                asset: QuoteAsset,
                fees,
            },
        }
    }

    pub fn asset_state(
        symbol: &str,
        balance_total: f64,
        balance_free: f64,
        time_exchange: DateTime<Utc>,
    ) -> AssetState {
        AssetState {
            asset: asset(symbol),
            balance: Balance::new(balance_total, balance_free),
            time_exchange,
        }
    }
}
