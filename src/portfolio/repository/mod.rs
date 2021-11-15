use crate::portfolio::position::Position;
use crate::portfolio::repository::error::RepositoryError;
use uuid::Uuid;

pub mod error;
pub mod in_memory;
pub mod redis;

/// Handles the reading & writing of a [Position] to/from the persistence layer.
pub trait PositionHandler {
    /// Upsert the [Position] at it's position_id.
    fn set_position(
        &mut self,
        portfolio_id: &Uuid,
        position: Position,
    ) -> Result<(), RepositoryError>;
    /// Get the [Position] using it's position_id.
    fn get_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError>;
    /// Remove the [Position] found at it's position_id.
    fn remove_position(
        &mut self,
        position_id: &String,
    ) -> Result<Option<Position>, RepositoryError>;
    /// Append a closed [Position] to the Portfolio's closed position list.
    fn set_closed_position(
        &mut self,
        portfolio_id: &Uuid,
        position: Position,
    ) -> Result<(), RepositoryError>;
    /// Get every closed [Position] associated with a Portfolio id.
    fn get_closed_positions(
        &mut self,
        portfolio_id: &Uuid,
    ) -> Result<Option<Vec<Position>>, RepositoryError>;
}

/// Handles the reading & writing of a Portfolio's current value to/from the persistence layer.
pub trait ValueHandler {
    /// Upsert the Portfolio current value as it's portfolio_id.
    fn set_current_value(&mut self, portfolio_id: &Uuid, value: f64)
        -> Result<(), RepositoryError>;
    /// Get the Portfolio current value using it's portfolio_id.
    fn get_current_value(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError>;
}

/// Handles the reading & writing of a Portfolio's current cash to/from the persistence layer.
pub trait CashHandler {
    /// Upsert the Portfolio current cash as it's portfolio_id.
    fn set_current_cash(&mut self, portfolio_id: &Uuid, cash: f64) -> Result<(), RepositoryError>;
    /// Get the Portfolio current cash using it's portfolio_id.
    fn get_current_cash(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError>;
}

/// Returns a [Position] unique identifier given a portfolio_id, exchange & symbol.
pub fn determine_position_id(portfolio_id: &Uuid, exchange: &String, symbol: &String) -> String {
    format!(
        "{}_{}_{}_{}",
        portfolio_id.to_string(),
        exchange,
        symbol,
        "position"
    )
}

/// Returns a unique identifier for a Portfolio's closed positions, given a portfolio_id.
pub fn determine_closed_positions_id(portfolio_id: &Uuid) -> String {
    format!("{}_closed_positions", portfolio_id.to_string())
}

/// Returns a unique identifier for a Portfolio's current value, given a portfolio_id.
pub fn determine_value_id(portfolio_id: &Uuid) -> String {
    format!("{}_value", portfolio_id.to_string())
}

/// Returns a unique identifier for a Portfolio's current cash, given a portfolio_id.\
pub fn determine_cash_id(portfolio_id: &Uuid) -> String {
    format!("{}_cash", portfolio_id.to_string())
}
