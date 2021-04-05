use data::{market::MarketEvent, handler::{Continuer, MarketGenerator, HistoricDataHandler}};
use strategy::{signal::SignalEvent, strategy::{SignalGenerator, RSIStrategy}};
use portfolio::{
    order::OrderEvent, position::Position,
    portfolio::{MarketUpdater, OrderGenerator, FillUpdater, PersistedMetaPortfolio},
    repository::redis::RedisRepository, allocator::OrderAllocator, risk::OrderEvaluator
};
use execution::{fill::FillEvent, handler::{FillGenerator, SimulatedExecution}};

/// Defines a [MarketEvent], and provides the useful traits of [Continuer] and [MarketGenerator] for
/// handling the generation of them. Contains implementations such as the [HistoricDataHandler] that
/// simulates a live market feed and acts as the systems heartbeat.
pub mod data;

/// Defines a [SignalEvent], and provides the [SignalGenerator] trait for handling the generation of
/// them. Contains an example [RSIStrategy] implementation that analyses a [MarketEvent] and may
/// generate a new [SignalEvent], an advisory signal for a Portfolio [OrderGenerator] to analyse.
pub mod strategy;

/// Defines useful data structures such as an [OrderEvent] and [Position]. The Portfolio must
/// interact with [MarketEvent]s, [SignalEvent]s, [OrderEvent]s, and [FillEvent]s. The useful traits
/// [MarketUpdater], [OrderGenerator], & [FillUpdater] are provided that define the interactions
/// with these events. Contains a [PersistedMetaPortfolio] implementation that persists state in a
/// [RedisRepository]. This also contains example implementations of a [OrderAllocator] &
/// [OrderEvaluator], and help the Portfolio make decisions on whether to generate [OrderEvent]s and
/// of what size.
pub mod portfolio;

/// Defines a [FillEvent], and provides a useful trait [FillGenerator] for handling the generation
/// of them. Contains a [SimulatedExecution] implementation that simulates a live broker execution
/// for the system.
pub mod execution;

