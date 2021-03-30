use uuid::Uuid;
use crate::portfolio::position::Position;
use crate::portfolio::repository::error::RepositoryError;

pub trait PositionHandler {
    fn set_position(&mut self, portfolio_id: &Uuid, position: &Position) -> Result<(), RepositoryError>;
    fn get_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError>;
    fn remove_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError>;
}

pub trait ValueHandler {
    fn set_current_value(&mut self, portfolio_id: &Uuid, value: f64)  -> Result<(), RepositoryError>;
    fn get_current_value(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError>;
}

pub trait CashHandler {
    fn set_current_cash(&mut self, portfolio_id: &Uuid, cash: f64)  -> Result<(), RepositoryError>;
    fn get_current_cash(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError>;
}