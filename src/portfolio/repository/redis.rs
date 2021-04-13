use uuid::Uuid;
use serde::Deserialize;
use crate::portfolio::position::Position;
use crate::portfolio::repository::error::RepositoryError;
use crate::portfolio::repository::error::RepositoryError::{
    JsonSerialisationError, WriteError, ReadError, JsonDeserialisationError, DeleteError};
use crate::portfolio::error::PortfolioError;
use crate::portfolio::error::PortfolioError::BuilderIncomplete;
use redis::{RedisResult, ErrorKind, Connection, Commands};

/// Handles the reading & writing of a [Position] to/from the persistence layer.
pub trait PositionHandler {
    /// Upsert the [Position] at it's position_id.
    fn set_position(&mut self, portfolio_id: &Uuid, position: &Position) -> Result<(), RepositoryError>;
    /// Get the [Position] using it's position_id.
    fn get_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError>;
    /// Remove the [Position] found at it's position_id.
    fn remove_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError>;
}

/// Handles the reading & writing of a Portfolio's current value to/from the persistence layer.
pub trait ValueHandler {
    /// Upsert the Portfolio current value as it's portfolio_id.
    fn set_current_value(&mut self, portfolio_id: &Uuid, value: f64)  -> Result<(), RepositoryError>;
    /// Get the Portfolio current value using it's portfolio_id.
    fn get_current_value(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError>;
}

/// Handles the reading & writing of a Portfolio's current cash to/from the persistence layer.
pub trait CashHandler {
    /// Upsert the Portfolio current cash as it's portfolio_id.
    fn set_current_cash(&mut self, portfolio_id: &Uuid, cash: f64)  -> Result<(), RepositoryError>;
    /// Get the Portfolio current cash using it's portfolio_id.
    fn get_current_cash(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError>;
}

/// Configuration for constructing a [RedisRepository] via the new() constructor method.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub uri: String
}

/// Redis persisted repository that implements [PositionHandler], [ValueHandler] & [CashHandler].
/// Used by a Portfolio implementation to persist the Portfolio state, including current value,
/// cash & Positions.
pub struct RedisRepository {
    conn: Connection,
}

impl PositionHandler for RedisRepository {
    fn set_position(&mut self, portfolio_id: &Uuid, position: &Position) -> Result<(), RepositoryError> {
        let position_key = determine_position_id(portfolio_id, &position.exchange, &position.symbol);

        let position_value = match serde_json::to_string(position) {
            Ok(position_str) => position_str,
            Err(_) => return Err(JsonSerialisationError())
        };

        let write_result: RedisResult<()> = self.conn.set(position_key, position_value);

        match write_result {
            Ok(_) => Ok(()),
            Err(_) => Err(WriteError())
        }
    }

    fn get_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
        let read_result: RedisResult<String> = self.conn.get(position_id);

        let position_value = match read_result {
            Ok(position_value) => position_value,
            Err(err) => {
                return match err.kind() {
                    ErrorKind::TypeError => Ok(None),
                    _ => Err(ReadError())
                }
            }
        };

        match serde_json::from_str(&*position_value) {
            Ok(position) => Ok(Some(position)),
            Err(_) => Err(JsonDeserialisationError())
        }
    }

    fn remove_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
        let position = self.get_position(position_id)?;

        let remove_result: RedisResult<()> = self.conn.del(position_id);

        match remove_result {
            Ok(_) => Ok(position),
            Err(_) => Err(DeleteError())
        }
    }
}

impl ValueHandler for RedisRepository {
    fn set_current_value(&mut self, portfolio_id: &Uuid, value: f64) -> Result<(), RepositoryError> {
        let current_value_key = determine_value_id(portfolio_id);
        let write_result: RedisResult<()> = self.conn.set(current_value_key, value);

        match write_result {
            Ok(_) => Ok(()),
            Err(_) => Err(WriteError())
        }
    }

    fn get_current_value(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
        let current_value_key = determine_value_id(portfolio_id);
        RedisRepository::parse_read_f64_result(self.conn.get(current_value_key))
    }
}

impl CashHandler for RedisRepository {
    fn set_current_cash(&mut self, portfolio_id: &Uuid, cash: f64) -> Result<(), RepositoryError> {
        let current_cash_key = determine_cash_id(portfolio_id);
        let write_result: RedisResult<()> = self.conn.set(current_cash_key, cash);

        match write_result {
            Ok(_) => Ok(()),
            Err(_) => Err(WriteError())
        }
    }

    fn get_current_cash(&mut self, portfolio_id: &Uuid) -> Result<f64, RepositoryError> {
        let current_cash_key = determine_cash_id(portfolio_id);
        RedisRepository::parse_read_f64_result(self.conn.get(current_cash_key))
    }
}

impl RedisRepository {
    /// Constructs a new [RedisRepository] component using the provided configuration struct.
    pub fn new(connection: Connection) -> RedisRepository {
        RedisRepository::builder()
            .conn(connection)
            .build()
            .expect("Failed to build RedisRepository")
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

    /// Parses a f64 [RedisResult] and returns a regular Result<f64>.
    pub fn parse_read_f64_result(result: RedisResult<f64>) -> Result<f64, RepositoryError> {
        match result {
            Ok(f64_value) => Ok(f64_value),
            Err(_) => Err(ReadError())
        }
    }
}

/// Builder to construct [RedisRepository] instances.
pub struct RedisRepositoryBuilder {
    conn: Option<Connection>,
}

impl RedisRepositoryBuilder {
    pub fn new() -> Self {
        Self {
            conn: None
        }
    }

    pub fn conn(mut self, value: Connection) -> Self {
        self.conn = Some(value);
        self
    }

    pub fn build(self) -> Result<RedisRepository, PortfolioError> {
        if let Some(conn) = self.conn {
            Ok(RedisRepository {
                conn
            })
        } else {
            Err(BuilderIncomplete())
        }
    }
}

/// Returns a [Position] unique identifier given a portfolio_id, exchange & symbol.
pub fn determine_position_id(portfolio_id: &Uuid, exchange: &String, symbol: &String) -> String {
    format!("{}_{}_{}", portfolio_id.to_string(), exchange, symbol)
}

/// Returns a unique identifier for a Portfolio's current value, given a portfolio_id.
pub fn determine_value_id(portfolio_id: &Uuid) -> String {
    format!("{}_value", portfolio_id.to_string())
}

/// Returns a unique identifier for a Portfolio's current cash, given a portfolio_id.
pub fn determine_cash_id(portfolio_id: &Uuid) -> String {
    format!("{}_cash", portfolio_id.to_string())
}