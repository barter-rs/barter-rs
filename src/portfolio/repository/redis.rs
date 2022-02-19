use crate::portfolio::error::PortfolioError;
use crate::portfolio::position::{determine_position_id, Position, PositionId};
use crate::portfolio::repository::error::RepositoryError;
use crate::portfolio::repository::{
    determine_cash_id, determine_equity_id, determine_exited_positions_id, AvailableCash,
    CashHandler, EquityHandler, PositionHandler, StatisticHandler, TotalEquity,
};
use crate::statistic::summary::PositionSummariser;
use crate::{Market, MarketId};
use redis::{Commands, Connection, ErrorKind};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use uuid::Uuid;

/// Configuration for constructing a [`RedisRepository`] via the new() constructor method.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub uri: String,
}

/// Redis persisted repository that implements [`PositionHandler`], [`ValueHandler`], [`CashHandler`]
/// & [`PositionSummariser`]. Used by a Portfolio implementation to persist the Portfolio state,
/// including total equity, available cash & Positions.
pub struct RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    conn: Connection,
    _statistic_marker: PhantomData<Statistic>,
}

impl<Statistic> PositionHandler for RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    fn set_open_position(&mut self, position: Position) -> Result<(), RepositoryError> {
        let position_string = serde_json::to_string(&position)?;

        self.conn
            .set(position.position_id, position_string)
            .map_err(|_| RepositoryError::WriteError)
    }

    fn get_open_position(
        &mut self,
        position_id: &PositionId,
    ) -> Result<Option<Position>, RepositoryError> {
        let position_value: String = self
            .conn
            .get(position_id)
            .map_err(|_| RepositoryError::ReadError)?;

        Ok(Some(serde_json::from_str::<Position>(&position_value)?))
    }

    fn get_open_positions<'a, Markets: Iterator<Item = &'a Market>>(
        &mut self,
        engine_id: &Uuid,
        markets: Markets,
    ) -> Result<Vec<Position>, RepositoryError> {
        markets
            .filter_map(|market| {
                self.get_open_position(&determine_position_id(
                    engine_id,
                    market.exchange,
                    &market.symbol,
                ))
                .transpose()
            })
            .collect()
    }

    fn remove_position(
        &mut self,
        position_id: &String,
    ) -> Result<Option<Position>, RepositoryError> {
        let position = self.get_open_position(position_id)?;

        self.conn
            .del(position_id)
            .map_err(|_| RepositoryError::DeleteError)?;

        Ok(position)
    }

    fn set_exited_position(
        &mut self,
        portfolio_id: &Uuid,
        position: Position,
    ) -> Result<(), RepositoryError> {
        self.conn
            .lpush(
                determine_exited_positions_id(portfolio_id),
                serde_json::to_string(&position)?
            )
            .map_err(|_| RepositoryError::WriteError)
    }

    fn get_exited_positions(
        &mut self,
        portfolio_id: &Uuid,
    ) -> Result<Vec<Position>, RepositoryError> {
        self.conn
            .get(determine_exited_positions_id(portfolio_id))
            .or_else(|err| match err.kind() {
                ErrorKind::TypeError => Ok(Vec::<String>::new()),
                _ => Err(RepositoryError::ReadError)
            })?
            .iter()
            .map(|position| serde_json::from_str::<Position>(position))
            .collect::<Result<Vec<Position>, serde_json::Error>>()
            .map_err(RepositoryError::JsonSerDeError)
    }
}

impl<Statistic> EquityHandler for RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    fn set_total_equity(
        &mut self,
        portfolio_id: &Uuid,
        equity: TotalEquity,
    ) -> Result<(), RepositoryError> {
        self.conn
            .set(determine_equity_id(portfolio_id), equity)
            .map_err(|_| RepositoryError::WriteError)
    }

    fn get_total_equity(&mut self, portfolio_id: &Uuid) -> Result<TotalEquity, RepositoryError> {
        self.conn
            .get(determine_equity_id(portfolio_id))
            .map_err(|_| RepositoryError::ReadError)
    }
}

impl<Statistic> CashHandler for RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    fn set_available_cash(
        &mut self,
        portfolio_id: &Uuid,
        cash: AvailableCash,
    ) -> Result<(), RepositoryError> {
        self.conn
            .set(determine_cash_id(portfolio_id), cash)
            .map_err(|_| RepositoryError::WriteError)
    }

    fn get_available_cash(
        &mut self,
        portfolio_id: &Uuid,
    ) -> Result<AvailableCash, RepositoryError> {
        self.conn
            .get(determine_cash_id(portfolio_id))
            .map_err(|_| RepositoryError::ReadError)
    }
}

impl<Statistic> StatisticHandler<Statistic> for RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    fn set_statistics(
        &mut self,
        market_id: &MarketId,
        statistic: Statistic,
    ) -> Result<(), RepositoryError> {
        self.conn
            .set(market_id, serde_json::to_string(&statistic)?)
            .map_err(|_| RepositoryError::WriteError)
    }

    fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError> {
        let statistics: String = self
            .conn
            .get(market_id)
            .map_err(|_| RepositoryError::ReadError)?;

        serde_json::from_str(&statistics).map_err(RepositoryError::JsonSerDeError)
    }
}

impl<Statistic: PositionSummariser> Debug for RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisRepository").finish()
    }
}

impl<Statistic: PositionSummariser> RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    /// Constructs a new [`RedisRepository`] component using the provided Redis connection struct.
    pub fn new(connection: Connection) -> Self {
        Self {
            conn: connection,
            _statistic_marker: PhantomData::<Statistic>::default(),
        }
    }

    /// Returns a [`RedisRepositoryBuilder`] instance.
    pub fn builder() -> RedisRepositoryBuilder<Statistic> {
        RedisRepositoryBuilder::new()
    }

    /// Establish & return a Redis connection.
    pub fn setup_redis_connection(cfg: Config) -> Connection {
        redis::Client::open(cfg.uri)
            .expect("Failed to create Redis client")
            .get_connection()
            .expect("Failed to connect to Redis")
    }
}

/// Builder to construct [`RedisRepository`] instances.
#[derive(Default)]
pub struct RedisRepositoryBuilder<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    conn: Option<Connection>,
    _statistic_marker: PhantomData<Statistic>,
}

impl<Statistic: PositionSummariser> RedisRepositoryBuilder<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    pub fn new() -> Self {
        Self {
            conn: None,
            _statistic_marker: PhantomData::<Statistic>::default(),
        }
    }

    pub fn conn(self, value: Connection) -> Self {
        Self {
            conn: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<RedisRepository<Statistic>, PortfolioError> {
        let conn = self.conn.ok_or(PortfolioError::BuilderIncomplete)?;

        Ok(RedisRepository {
            conn,
            _statistic_marker: PhantomData::<Statistic>::default(),
        })
    }
}

impl<Statistic> Debug for RedisRepositoryBuilder<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisRepositoryBuilder")
            .field("conn", &"Option<redis::Connection>")
            .field("_statistic_market", &self._statistic_marker)
            .finish()
    }
}