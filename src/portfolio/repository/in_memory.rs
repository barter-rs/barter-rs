use crate::portfolio::repository::redis::{PositionHandler, ValueHandler, CashHandler, determine_position_id};
use uuid::Uuid;
use crate::portfolio::position::Position;
use crate::portfolio::repository::error::RepositoryError;
use std::collections::HashMap;

/// In-Memory repository that implements [PositionHandler], [ValueHandler] & [CashHandler].
/// Used by a Portfolio implementation to save the Portfolio state without fault tolerant guarantees.
/// State includes current value, cash & Positions.
pub struct InMemoryRepository {
    open_positions: HashMap<String, Position>,
    closed_positions: HashMap<String, Position>,
    current_values: HashMap<String, f64>,
    current_cashes: HashMap<String, f64>,
}

impl PositionHandler for InMemoryRepository {
    fn set_position(&mut self, portfolio_id: &Uuid, position: &Position) -> Result<(), RepositoryError> {
        todo!()
    }

    fn get_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
        todo!()
    }

    fn remove_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
        todo!()
    }

    fn set_closed_position(&mut self, portfolio_id: &Uuid, position: &Position) -> Result<(), RepositoryError> {
        todo!()
    }
}

impl ValueHandler for InMemoryRepository {
    fn set_current_value(&mut self, portfolio_id: &Uuid, value: f64) -> Result<(), RepositoryError> {
        todo!()
    }

    fn get_current_value(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
        todo!()
    }
}

impl CashHandler for InMemoryRepository {
    fn set_current_cash(&mut self, portfolio_id: &Uuid, cash: f64) -> Result<(), RepositoryError> {
        todo!()
    }

    fn get_current_cash(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
        todo!()
    }
}