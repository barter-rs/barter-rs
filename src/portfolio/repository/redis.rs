use crate::{
    portfolio::{
        error::PortfolioError,
        position::{determine_position_id, Position, PositionId},
        repository::{
            determine_exited_positions_id, error::RepositoryError, BalanceHandler, PositionHandler,
            StatisticHandler,
        },
        Balance,
    },
    statistic::summary::PositionSummariser,
};
use barter_integration::model::{Market, MarketId};
use redis::{Commands, Connection, ErrorKind};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
};
use uuid::Uuid;

/// Configuration for constructing a [`RedisRepository`] via the new() constructor method.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub uri: String,
}

/// Redis persisted repository that implements [`PositionHandler`], [`BalanceHandler`],
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
        engine_id: Uuid,
        markets: Markets,
    ) -> Result<Vec<Position>, RepositoryError> {
        markets
            .filter_map(|market| {
                self.get_open_position(&determine_position_id(
                    engine_id,
                    &market.exchange,
                    &market.instrument,
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
        engine_id: Uuid,
        position: Position,
    ) -> Result<(), RepositoryError> {
        self.conn
            .lpush(
                determine_exited_positions_id(engine_id),
                serde_json::to_string(&position)?,
            )
            .map_err(|_| RepositoryError::WriteError)
    }

    fn get_exited_positions(&mut self, engine_id: Uuid) -> Result<Vec<Position>, RepositoryError> {
        self.conn
            .get(determine_exited_positions_id(engine_id))
            .or_else(|err| match err.kind() {
                ErrorKind::TypeError => Ok(Vec::<String>::new()),
                _ => Err(RepositoryError::ReadError),
            })?
            .iter()
            .map(|position| serde_json::from_str::<Position>(position))
            .collect::<Result<Vec<Position>, serde_json::Error>>()
            .map_err(RepositoryError::JsonSerDeError)
    }
}

impl<Statistic> BalanceHandler for RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    fn set_balance(&mut self, engine_id: Uuid, balance: Balance) -> Result<(), RepositoryError> {
        let balance_string = serde_json::to_string(&balance)?;

        self.conn
            .set(Balance::balance_id(engine_id), balance_string)
            .map_err(|_| RepositoryError::WriteError)
    }

    fn get_balance(&mut self, engine_id: Uuid) -> Result<Balance, RepositoryError> {
        let balance_value: String = self
            .conn
            .get(Balance::balance_id(engine_id))
            .map_err(|_| RepositoryError::ReadError)?;

        Ok(serde_json::from_str::<Balance>(&balance_value)?)
    }
}

impl<Statistic> StatisticHandler<Statistic> for RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    fn set_statistics(
        &mut self,
        market_id: MarketId,
        statistic: Statistic,
    ) -> Result<(), RepositoryError> {
        self.conn
            .set(market_id.0, serde_json::to_string(&statistic)?)
            .map_err(|_| RepositoryError::WriteError)
    }

    fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError> {
        let statistics: String = self
            .conn
            .get(&market_id.0)
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
        Ok(RedisRepository {
            conn: self.conn.ok_or(PortfolioError::BuilderIncomplete("conn"))?,
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
