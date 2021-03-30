use uuid::Uuid;
use crate::portfolio::position::Position;
use crate::portfolio::repository::error::RepositoryError;
use crate::portfolio::repository::error::RepositoryError::{JsonSerialisationError, WriteError, ReadError, JsonDeserialisationError, DeleteError};
use crate::portfolio::error::PortfolioError;
use crate::portfolio::error::PortfolioError::BuilderIncomplete;

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

#[derive(Debug, Deserialize)]
pub struct Config {
    pub uri: String
}

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
    pub fn new(connection: Connection) -> RedisRepository {
        RedisRepository::builder()
            .conn(connection)
            .build()
            .expect("Failed to build RedisRepository")
    }

    pub fn builder() -> RedisRepositoryBuilder {
        RedisRepositoryBuilder::new()
    }

    pub fn setup_redis_connection(cfg: &Config) -> Connection {
        redis::Client::open(&*cfg.uri)
            .expect("Failed to create Redis client")
            .get_connection()
            .expect("Failed to connect to Redis")
    }

    pub fn parse_read_f64_result(result: RedisResult<f64>) -> Result<f64, RepositoryError> {
        match result {
            Ok(f64_value) => Ok(f64_value),
            Err(_) => Err(ReadError())
        }
    }
}

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

pub fn determine_position_id(portfolio_id: &Uuid, exchange: &String, symbol: &String) -> String {
    format!("{}_{}_{}", portfolio_id.to_string(), exchange, symbol)
}

pub fn determine_value_id(portfolio_id: &Uuid) -> String {
    format!("{}_value", portfolio_id.to_string())
}

pub fn determine_cash_id(portfolio_id: &Uuid) -> String {
    format!("{}_cash", portfolio_id.to_string())
}