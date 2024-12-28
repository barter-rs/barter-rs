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
//! Todo: write docs
//! ### Engine Examples
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
pub mod logging;
pub mod risk;
pub mod statistic;
pub mod strategy;

// Todo: Must: Final Requirements
//  - Comprehensive rust docs & check output
//  - Comprehensive rust examples
//  - Comprehensive readme.md for each crate & workspace
//  - Comprehensive rust tests

// Todo: MarketData:
// Todo: add utils for creating Instruments from basic base,quote,kind,exchange, etc.

pub type FnvIndexMap<K, V> = indexmap::IndexMap<K, V, fnv::FnvBuildHasher>;
pub type FnvIndexSet<T> = indexmap::IndexSet<T, fnv::FnvBuildHasher>;

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
    value: T,
    time: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, From)]
pub enum EngineEvent<
    MarketKind,
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
> {
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

#[cfg(test)]
pub mod test_utils {
    use crate::{
        engine::state::asset::AssetState, statistic::summary::asset::TearSheetAssetGenerator, Timed,
    };
    use barter_execution::{
        balance::Balance,
        order::{OrderId, StrategyId},
        trade::{AssetFees, Trade, TradeId},
    };
    use barter_instrument::{
        asset::QuoteAsset, instrument::name::InstrumentNameInternal, test_utils::asset, Side,
    };
    use chrono::{DateTime, Days, Utc};
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
