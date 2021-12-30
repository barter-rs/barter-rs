use crate::portfolio::position::{Position, PositionId};
use crate::portfolio::repository::error::RepositoryError;
use crate::{Market, MarketId};
use uuid::Uuid;

pub mod error;
pub mod in_memory;
pub mod redis;

/// Handles the reading & writing of a [`Position`] to/from the persistence layer.
pub trait PositionHandler {
    /// Upsert the open [`Position`] using it's [`PositionId`].
    fn set_open_position(&mut self, position: Position) -> Result<(), RepositoryError>;

    /// Get an open [`Position`] using the [`PositionId`] provided.
    fn get_open_position(&mut self, position_id: &PositionId) -> Result<Option<Position>, RepositoryError>;

    /// Get all open [`Position`]s associated with a Portfolio.
    fn get_open_positions(&mut self, engine_id: &Uuid, markets: &Vec<Market>) -> Result<Vec<Position>, RepositoryError>;

    /// Remove the [`Position`] at the [`PositionId`].
    fn remove_position(&mut self, position_id: &PositionId) -> Result<Option<Position>, RepositoryError>;

    /// Append an exited [`Position`] to the Portfolio's exited position list.
    fn set_exited_position(&mut self, engine_id: &Uuid, position: Position) -> Result<(), RepositoryError>;

    /// Get every exited [`Position`] associated with the engine_id.
    fn get_exited_positions(&mut self, engine_id: &Uuid) -> Result<Option<Vec<Position>>, RepositoryError>;
}

/// Handles the reading & writing of a Portfolio's current total equity value to/from the persistence layer.
pub trait EquityHandler {
    /// Upsert the Portfolio [`TotalEquity`] at the engine_id.
    fn set_total_equity(&mut self, engine_id: &Uuid, value: TotalEquity) -> Result<(), RepositoryError>;
    /// Get the Portfolio [`TotalEquity`] using the engine_id provided.
    fn get_total_equity(&mut self, engine_id: &Uuid) -> Result<TotalEquity, RepositoryError>;
}

/// Handles the reading & writing of a Portfolio's current available cash to/from the persistence layer.
pub trait CashHandler {
    /// Upsert the Portfolio current cash at the engine_id.
    fn set_available_cash(&mut self, engine_id: &Uuid, cash: AvailableCash) -> Result<(), RepositoryError>;
    /// Get the Portfolio current cash using the engine_id provided.
    fn get_available_cash(&mut self, engine_id: &Uuid) -> Result<AvailableCash, RepositoryError>;
}

/// Handles the reading & writing of a Portfolio's statistics for each of it's
/// markets, where each market is represented by a [`MarketId`].
pub trait StatisticHandler<Statistic> {
    /// Upsert the market statistics at the market_id provided.
    fn set_statistics(&mut self, market_id: &MarketId, statistic: Statistic) -> Result<(), RepositoryError>;
    /// Get the market statistics using the market_id provided.
    fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError>;
}

/// Communicates a String represents a unique identifier for all a Portfolio's exited [`Position`]s.
/// Used to append new exited [`Position`]s to the entry in the [`PositionHandler`].
pub type ExitedPositionsId = String;

/// Returns the unique identifier for a Portfolio's exited [`Position`]s, given an engine_id.
pub fn determine_exited_positions_id(engine_id: &Uuid) -> ExitedPositionsId {
    format!("positions_exited_{}", engine_id)
}

/// Communicates a String represents a unique identifier for a [`TotalEquity`].
pub type EquityId = String;

/// Communicates an f64 represents total equity.
pub type TotalEquity = f64;

/// Returns the unique identifier for a Portfolio's current [`TotalEquity`], given an engine_id.
pub fn determine_equity_id(engine_id: &Uuid) -> String {
    format!("{}_equity", engine_id)
}

/// Communicates a String represents a unique identifier for an [`AvailableCash`].
pub type CashId = String;

/// Communicates an f64 represents available cash.
pub type AvailableCash = f64;

/// Returns the unique identifier for a Portfolio's [`AvailableCash`], given an engine_id.
pub fn determine_cash_id(engine_id: &Uuid) -> String {
    format!("{}_cash", engine_id)
}