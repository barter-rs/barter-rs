use crate::portfolio::error::PortfolioError;
use crate::portfolio::position::Position;
use crate::portfolio::repository::error::RepositoryError;
use crate::portfolio::repository::{
    determine_cash_id, determine_closed_positions_id, determine_position_id, determine_value_id,
    CashHandler, PositionHandler, ValueHandler,
};
use redis::{Commands, Connection, ErrorKind, RedisResult};
use serde::Deserialize;
use std::fmt::{Debug, Formatter};
use uuid::Uuid;

/// Configuration for constructing a [RedisRepository] via the new() constructor method.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub uri: String,
}

/// Redis persisted repository that implements [PositionHandler], [ValueHandler] & [CashHandler].
/// Used by a Portfolio implementation to persist the Portfolio state, including current value,
/// cash & Positions.
pub struct RedisRepository {
    conn: Connection,
}

impl Debug for RedisRepository {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisRepository").finish()
    }
}

impl PositionHandler for RedisRepository {
    fn set_position(
        &mut self,
        portfolio_id: &Uuid,
        position: Position,
    ) -> Result<(), RepositoryError> {
        let position_key =
            determine_position_id(portfolio_id, &position.exchange, &position.symbol);

        let position_value = serde_json::to_string(&position)
            .map_err(|_err| RepositoryError::JsonSerialisationError)?;

        let result = self
            .conn
            .set(position_key, position_value)
            .map_err(|_err| RepositoryError::WriteError)?;

        Ok(result)
    }

    fn get_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
        let read_result: RedisResult<String> = self.conn.get(position_id);

        let position_value = match read_result {
            Ok(position_value) => position_value,
            Err(err) => {
                return match err.kind() {
                    ErrorKind::TypeError => Ok(None),
                    _ => Err(RepositoryError::ReadError),
                }
            }
        };

        match serde_json::from_str(&*position_value) {
            Ok(position) => Ok(Some(position)),
            Err(_) => Err(RepositoryError::JsonDeserialisationError),
        }
    }

    fn remove_position(
        &mut self,
        position_id: &String,
    ) -> Result<Option<Position>, RepositoryError> {
        let position = self.get_position(position_id)?;

        self.conn
            .del(position_id)
            .map_err(|_err| RepositoryError::DeleteError)?;

        Ok(position)
    }

    fn set_closed_position(
        &mut self,
        portfolio_id: &Uuid,
        position: Position,
    ) -> Result<(), RepositoryError> {
        let closed_positions_key = determine_closed_positions_id(&portfolio_id);

        let position_value = serde_json::to_string(&position)
            .map_err(|_err| RepositoryError::JsonSerialisationError)?;

        let result = self
            .conn
            .lpush(closed_positions_key, position_value)
            .map_err(|_err| RepositoryError::WriteError)?;

        Ok(result)
    }

    fn get_closed_positions(
        &mut self,
        portfolio_id: &Uuid,
    ) -> Result<Option<Vec<Position>>, RepositoryError> {
        let closed_positions_key = determine_closed_positions_id(portfolio_id);

        let read_result: RedisResult<Vec<String>> = self.conn.get(closed_positions_key);

        let closed_positions = match read_result {
            Ok(closed_positions_value) => closed_positions_value,
            Err(err) => {
                return match err.kind() {
                    ErrorKind::TypeError => Ok(None),
                    _ => Err(RepositoryError::ReadError),
                }
            }
        }
        .iter()
        .map(|positions_string| serde_json::from_str(positions_string).unwrap())
        .collect();

        Ok(Some(closed_positions))
    }
}

impl ValueHandler for RedisRepository {
    fn set_current_value(
        &mut self,
        portfolio_id: &Uuid,
        value: f64,
    ) -> Result<(), RepositoryError> {
        let current_value_key = determine_value_id(portfolio_id);

        let result = self
            .conn
            .set(current_value_key, value)
            .map_err(|_err| RepositoryError::WriteError)?;

        Ok(result)
    }

    fn get_current_value(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
        let current_value_key = determine_value_id(portfolio_id);

        let current_value: f64 = self
            .conn
            .get(current_value_key)
            .map_err(|_err| RepositoryError::ReadError)?;

        Ok(current_value)
    }
}

impl CashHandler for RedisRepository {
    fn set_current_cash(&mut self, portfolio_id: &Uuid, cash: f64) -> Result<(), RepositoryError> {
        let current_cash_key = determine_cash_id(portfolio_id);

        let result = self
            .conn
            .set(current_cash_key, cash)
            .map_err(|_err| RepositoryError::WriteError)?;

        Ok(result)
    }

    fn get_current_cash(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
        let current_cash_key = determine_cash_id(portfolio_id);

        let current_cash = self
            .conn
            .get(current_cash_key)
            .map_err(|_err| RepositoryError::ReadError)?;

        Ok(current_cash)
    }
}

impl RedisRepository {
    /// Constructs a new [RedisRepository] component using the provided Redis connection struct.
    pub fn new(connection: Connection) -> Self {
        Self { conn: connection }
    }

    /// Returns a [RedisRepositoryBuilder] instance.
    pub fn builder() -> RedisRepositoryBuilder {
        RedisRepositoryBuilder::new()
    }

    /// Establish & return a Redis connection.
    pub fn setup_redis_connection(cfg: &Config) -> Connection {
        redis::Client::open(&*cfg.uri)
            .expect("Failed to create Redis client")
            .get_connection()
            .expect("Failed to connect to Redis")
    }
}

/// Builder to construct [RedisRepository] instances.
#[derive(Default)]
pub struct RedisRepositoryBuilder {
    conn: Option<Connection>,
}

impl RedisRepositoryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn conn(self, value: Connection) -> Self {
        Self {
            conn: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<RedisRepository, PortfolioError> {
        let conn = self.conn.ok_or(PortfolioError::BuilderIncomplete)?;

        Ok(RedisRepository { conn })
    }
}
