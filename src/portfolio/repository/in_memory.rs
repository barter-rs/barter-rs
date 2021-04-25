use crate::portfolio::repository::redis::{PositionHandler, ValueHandler, CashHandler, determine_position_id, determine_closed_positions_id, determine_value_id, determine_cash_id};
use uuid::Uuid;
use crate::portfolio::position::Position;
use crate::portfolio::repository::error::RepositoryError;
use std::collections::HashMap;

/// In-Memory repository for Proof Of Concepts. Implements [PositionHandler], [ValueHandler] &
/// [CashHandler]. Used by a Proof Of Concept Portfolio implementation to save the current value,
/// cash & Positions. **Do not use in production - no fault tolerant guarantees!**
pub struct InMemoryRepository {
    open_positions: HashMap<String, Position>,
    closed_positions: HashMap<String, Vec<Position>>,
    current_values: HashMap<String, f64>,
    current_cashes: HashMap<String, f64>,
}

impl PositionHandler for InMemoryRepository {
    fn set_position(&mut self, portfolio_id: &Uuid, position: Position) -> Result<(), RepositoryError> {
        let position_key = determine_position_id(portfolio_id, &position.exchange, &position.symbol);

        self.open_positions.insert(position_key, position);
        Ok(())
    }

    fn get_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
        let position = match self.open_positions.remove(position_id) {
            None => return Ok(None),
            Some(position) => position,
        };

        self.open_positions.insert(position_id.clone(), position.clone());

        Ok(Some(position))
    }

    fn remove_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
        Ok(self.open_positions.remove(position_id))
    }

    fn set_closed_position(&mut self, portfolio_id: &Uuid, position: Position) -> Result<(), RepositoryError> {
        let closed_positions_key = determine_closed_positions_id(portfolio_id);

        match self.closed_positions.get_mut(&*closed_positions_key) {
            None => {
                let mut new_closed_positions = Vec::new();
                new_closed_positions.push(position);
                self.closed_positions.insert(closed_positions_key, new_closed_positions);
            },
            Some(closed_positions) => {
                closed_positions.push(position)
            }
        }

        Ok(())
    }

    fn get_closed_positions(&mut self, portfolio_id: &Uuid) -> Result<Option<Vec<Position>>, RepositoryError> {
        let closed_positions_key = determine_closed_positions_id(portfolio_id);

        Ok(self.closed_positions.remove(&*closed_positions_key))
    }
}

impl ValueHandler for InMemoryRepository {
    fn set_current_value(&mut self, portfolio_id: &Uuid, value: f64) -> Result<(), RepositoryError> {
        let current_value_key = determine_value_id(portfolio_id);

        self.current_values.insert(current_value_key, value);

        Ok(())
    }

    fn get_current_value(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
        let current_value_key = determine_value_id(portfolio_id);

        match self.current_values.get(&*current_value_key) {
            None => Err(RepositoryError::ExpectedDataNotPresentError),
            Some(value) => Ok(*value)
        }
    }
}

impl CashHandler for InMemoryRepository {
    fn set_current_cash(&mut self, portfolio_id: &Uuid, cash: f64) -> Result<(), RepositoryError> {
        let current_cash_key = determine_cash_id(portfolio_id);

        self.current_cashes.insert(current_cash_key, cash);

        Ok(())
    }

    fn get_current_cash(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
        let current_cash_key = determine_cash_id(portfolio_id);

        match self.current_cashes.get(&*current_cash_key) {
            None => Err(RepositoryError::ExpectedDataNotPresentError),
            Some(cash) => Ok(*cash)
        }
    }
}

impl InMemoryRepository {
    /// Constructs a new [InMemoryRepository] component.
    pub fn new() -> Self {
        Self {
            open_positions: HashMap::new(),
            closed_positions: HashMap::new(),
            current_values: HashMap::new(),
            current_cashes: HashMap::new()
        }
    }
}