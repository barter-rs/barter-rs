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
//! Barter core is a Rust framework for building high-performance live-trading, paper-trading and back-testing systems.
//! * **Fast**: Written in native Rust. Minimal allocations. Data-oriented state management system with direct index lookups.
//! * **Robust**: Strongly typed. Thread safe. Extensive test coverage.
//! * **Customisable**: Plug and play Strategy and RiskManager components that facilitates most trading strategies (MarketMaking, StatArb, HFT, etc.).
//! * **Scalable**: Multithreaded architecture with modular design. Leverages Tokio for I/O. Memory efficient data structures.
//!
//! ## Overview
//! Barter core is a Rust framework for building professional grade live-trading, paper-trading and back-testing systems. The
//! central Engine facilitates executing on many exchanges simultaneously, and offers the flexibility to run most types of
//! trading strategies.  It allows turning algorithmic order generation on/off and can action Commands issued from external
//! processes (eg/ CloseAllPositions, OpenOrders, CancelOrders, etc.)
//!
//! At a high-level, it provides a few major components:
//! * `Engine` with plug and play `Strategy` and `RiskManager` components.
//! * Centralised cache friendly `EngineState` management with O(1) constant lookups using indexed data structures.
//! * `Strategy` interfaces for customising Engine behavior (AlgoStrategy, ClosePositionsStrategy, OnDisconnectStrategy, etc.).
//! * `RiskManager` interface for defining custom risk logic which checking generated algorithmic orders.
//! * Event-driven system that allows for Commands to be issued from external processes (eg/ CloseAllPositions, OpenOrders, CancelOrders, etc.),
//!   as well as turning algorithmic trading on/off.
//! * Comprehensive statistics package that provides a summary of key performance metrics (PnL, Sharpe, Sortino, Drawdown, etc.).
//!
//! ## Getting Started Via Engine Examples
//! [See Engine Examples](https://github.com/barter-rs/barter-rs/tree/feat/docs_tests_readmes_examples/barter/examples)

use crate::{
    engine::{command::Command, state::trading::TradingState},
    execution::AccountStreamEvent,
};
use barter_data::{
    event::{DataKind, MarketEvent},
    streams::consumer::MarketStreamEvent,
};
use barter_execution::AccountEvent;
use barter_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use barter_integration::Terminal;
use chrono::{DateTime, Utc};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use shutdown::Shutdown;

/// Algorithmic trading `Engine`, and entry points for processing input `Events`.
///
/// eg/ `Engine`, `run`, `process_with_audit`, etc.
pub mod engine;

/// Defines all possible errors in Barter core.
pub mod error;

/// Components for initialising multi-exchange execution, routing `ExecutionRequest`s and other
/// execution logic.
pub mod execution;

/// Provides default Barter core Tracing logging initialisers.
pub mod logging;

/// RiskManager interface for reviewing and optionally filtering algorithmic cancel and open
/// order requests.
pub mod risk;

/// Statistical algorithms for analysing datasets, financial metrics and financial summaries.
///
/// eg/ `TradingSummary`, `TearSheet`, `SharpeRatio`, etc.
pub mod statistic;

/// Strategy interfaces for generating algorithmic orders, closing positions, and performing
/// `Engine` actions on disconnect / trading disabled.
pub mod strategy;

/// Utilities for initialising and interacting with a full trading system.
pub mod system;

/// Backtesting utilities.
pub mod backtest;

/// Traits and types related to component shutdowns.
pub mod shutdown;

/// A timed value.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Deserialize,
    Serialize,
    Constructor,
)]
pub struct Timed<T> {
    pub value: T,
    pub time: DateTime<Utc>,
}

/// Default [`Engine`](engine::Engine) event that encompasses market events, account/execution
/// events, and `Engine` commands.
///
/// Note that the `Engine` can be configured to process custom events.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From)]
pub enum EngineEvent<
    MarketKind = DataKind,
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
> {
    Shutdown(Shutdown),
    Command(Command<ExchangeKey, AssetKey, InstrumentKey>),
    TradingStateUpdate(TradingState),
    Account(AccountStreamEvent<ExchangeKey, AssetKey, InstrumentKey>),
    Market(MarketStreamEvent<InstrumentKey, MarketKind>),
}

impl<MarketKind, ExchangeKey, AssetKey, InstrumentKey> Terminal
    for EngineEvent<MarketKind, ExchangeKey, AssetKey, InstrumentKey>
{
    fn is_terminal(&self) -> bool {
        matches!(self, Self::Shutdown(_))
    }
}

impl<MarketKind, ExchangeKey, AssetKey, InstrumentKey>
    EngineEvent<MarketKind, ExchangeKey, AssetKey, InstrumentKey>
{
    pub fn shutdown() -> Self {
        Self::Shutdown(Shutdown)
    }
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

/// Monotonically increasing event sequence. Used to track `Engine` event processing sequence.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Sequence(pub u64);

impl Sequence {
    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn fetch_add(&mut self) -> Sequence {
        let sequence = *self;
        self.0 += 1;
        sequence
    }
}

/// Barter core test utilities.
pub mod test_utils {
    use crate::{
        Timed, engine::state::asset::AssetState, statistic::summary::asset::TearSheetAssetGenerator,
    };
    use barter_execution::{
        balance::Balance,
        order::id::{OrderId, StrategyId},
        trade::{AssetFees, Trade, TradeId},
    };
    use barter_instrument::{
        Side, asset::QuoteAsset, instrument::name::InstrumentNameInternal, test_utils::asset,
    };
    use chrono::{DateTime, Days, TimeDelta, Utc};
    use rust_decimal::Decimal;

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

    pub fn time_plus_secs(base: DateTime<Utc>, plus: i64) -> DateTime<Utc> {
        base.checked_add_signed(TimeDelta::seconds(plus)).unwrap()
    }

    pub fn time_plus_millis(base: DateTime<Utc>, plus: i64) -> DateTime<Utc> {
        base.checked_add_signed(TimeDelta::milliseconds(plus))
            .unwrap()
    }

    pub fn time_plus_micros(base: DateTime<Utc>, plus: i64) -> DateTime<Utc> {
        base.checked_add_signed(TimeDelta::microseconds(plus))
            .unwrap()
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
            price: price.try_into().unwrap(),
            quantity: quantity.try_into().unwrap(),
            fees: AssetFees {
                asset: QuoteAsset,
                fees: fees.try_into().unwrap(),
            },
        }
    }

    pub fn asset_state(
        symbol: &str,
        balance_total: f64,
        balance_free: f64,
        time_exchange: DateTime<Utc>,
    ) -> AssetState {
        let balance = Timed::new(
            Balance::new(
                Decimal::try_from(balance_total).unwrap(),
                Decimal::try_from(balance_free).unwrap(),
            ),
            time_exchange,
        );

        AssetState {
            asset: asset(symbol),
            balance: Some(balance),
            statistics: TearSheetAssetGenerator::init(&balance),
        }
    }
}
