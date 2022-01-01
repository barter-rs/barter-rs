// use crate::portfolio::error::PortfolioError;
// use crate::portfolio::position::{determine_position_id, Position};
// use crate::portfolio::repository::error::RepositoryError;
// use crate::portfolio::repository::{determine_cash_id, CashHandler, PositionHandler, determine_exited_positions_id, EquityHandler, TotalEquity, determine_equity_id, AvailableCash, StatisticHandler};
// use redis::{Commands, Connection, ErrorKind, RedisResult};
// use serde::{Deserialize, Serialize};
// use std::fmt::{Debug, Formatter};
// use std::marker::PhantomData;
// use serde::de::DeserializeOwned;
// use uuid::Uuid;
// use crate::{Market, MarketId};
// use crate::statistic::summary::PositionSummariser;
//
// /// Configuration for constructing a [`RedisRepository`] via the new() constructor method.
// #[derive(Debug, Deserialize)]
// pub struct Config {
//     pub uri: String,
// }
//
// /// Redis persisted repository that implements [`PositionHandler`], [`ValueHandler`], [`CashHandler`]
// /// & [`PositionSummariser`]. Used by a Portfolio implementation to persist the Portfolio state,
// /// including total equity, available cash & Positions.
// pub struct RedisRepository<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned
// {
//     conn: Connection,
//     _statistic_marker: PhantomData<Statistic>,
// }
//
// impl<Statistic: PositionSummariser> Debug for RedisRepository<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned
// {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("RedisRepository").finish()
//     }
// }
//
// impl<Statistic> PositionHandler for RedisRepository<Statistic>
//     where
//         Statistic: PositionSummariser + Serialize + DeserializeOwned
// {
//     fn set_open_position(&mut self, position: Position) -> Result<(), RepositoryError> {
//         let position_value = serde_json::to_string(&position)
//             .map_err(|_err| RepositoryError::JsonSerialisationError)?;
//
//         Ok(self
//             .conn
//             .set(position.position_id, position_value)
//             .map_err(|_| RepositoryError::WriteError)?
//         )
//     }
//
//     fn get_open_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
//         let read_result: RedisResult<String> = self.conn.get(position_id);
//
//         let position_value = match read_result {
//             Ok(position_value) => position_value,
//             Err(_) => return Err(RepositoryError::ReadError)
//         };
//
//         match serde_json::from_str(&*position_value) {
//             Ok(position) => Ok(Some(position)),
//             Err(_) => Err(RepositoryError::JsonDeserialisationError),
//         }
//     }
//
//     fn get_open_positions(&mut self, engine_id: &Uuid, markets: &Vec<Market>) -> Result<Vec<Position>, RepositoryError> {
//         markets
//             .into_iter()
//             .filter_map(|market| {
//                 self.get_open_position(&determine_position_id(engine_id, market.exchange, &market.symbol))
//                     .transpose()
//             })
//             .collect()
//     }
//
//     fn remove_position(
//         &mut self,
//         position_id: &String,
//     ) -> Result<Option<Position>, RepositoryError> {
//         let position = self.get_open_position(position_id)?;
//
//         self.conn
//             .del(position_id)
//             .map_err(|_| RepositoryError::DeleteError)?;
//
//         Ok(position)
//     }
//
//     fn set_exited_position(
//         &mut self,
//         portfolio_id: &Uuid,
//         position: Position,
//     ) -> Result<(), RepositoryError> {
//         let closed_positions_key = determine_exited_positions_id(&portfolio_id);
//
//         let position_value = serde_json::to_string(&position)
//             .map_err(|_err| RepositoryError::JsonSerialisationError)?;
//
//         Ok(self
//             .conn
//             .lpush(closed_positions_key, position_value)
//             .map_err(|_| RepositoryError::WriteError)?
//         )
//     }
//
//     fn get_exited_positions(
//         &mut self,
//         portfolio_id: &Uuid,
//     ) -> Result<Option<Vec<Position>>, RepositoryError> {
//         let closed_positions_key = determine_exited_positions_id(portfolio_id);
//
//         let read_result: RedisResult<Vec<String>> = self.conn.get(closed_positions_key);
//
//         let closed_positions = match read_result {
//             Ok(closed_positions_value) => closed_positions_value,
//             Err(err) => {
//                 return match err.kind() {
//                     ErrorKind::TypeError => Ok(None),
//                     _ => Err(RepositoryError::ReadError),
//                 }
//             }
//         }
//             .iter()
//             .map(|positions_string| serde_json::from_str(positions_string).unwrap())
//             .collect();
//
//         Ok(Some(closed_positions))
//     }
// }
//
// impl<Statistic> EquityHandler for RedisRepository<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned
// {
//     fn set_total_equity(
//         &mut self,
//         portfolio_id: &Uuid,
//         equity: TotalEquity,
//     ) -> Result<(), RepositoryError> {
//         Ok(self
//             .conn
//             .set(determine_equity_id(portfolio_id), equity)
//             .map_err(|_| RepositoryError::WriteError)?
//         )
//     }
//
//     fn get_total_equity(&mut self, portfolio_id: &Uuid) -> Result<TotalEquity, RepositoryError> {
//         Ok(self
//             .conn
//             .get(determine_equity_id(portfolio_id))
//             .map_err(|_| RepositoryError::ReadError)?
//         )
//     }
// }
//
// impl<Statistic> CashHandler for RedisRepository<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned
// {
//     fn set_available_cash(&mut self, portfolio_id: &Uuid, cash: AvailableCash) -> Result<(), RepositoryError> {
//         Ok(self
//             .conn
//             .set(determine_cash_id(portfolio_id), cash)
//             .map_err(|_| RepositoryError::WriteError)?
//         )
//     }
//
//     fn get_available_cash(&mut self, portfolio_id: &Uuid) -> Result<AvailableCash, RepositoryError> {
//         Ok(self
//             .conn
//             .get(determine_cash_id(portfolio_id))
//             .map_err(|_| RepositoryError::ReadError)?
//         )
//     }
// }
//
// impl<Statistic> StatisticHandler<Statistic> for RedisRepository<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned
// {
//     fn set_statistics(&mut self, market_id: &MarketId, statistic: Statistic) -> Result<(), RepositoryError> {
//         let statistics_value = serde_json::to_string(&statistic)
//             .map_err(|_| RepositoryError::JsonSerialisationError)?;
//
//         Ok(self
//             .conn
//             .set(market_id, statistics_value)
//             .map_err(|_| RepositoryError::WriteError)?
//         )
//     }
//
//     fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError> {
//         let read_result: RedisResult<String> = self.conn.get(market_id);
//
//         let statistics_string = match read_result {
//             Ok(statistics_string) => statistics_string,
//             Err(_) => return Err(RepositoryError::ReadError)
//         };
//
//         serde_json::from_str(&statistics_string)
//             .map_err(|_| RepositoryError::JsonDeserialisationError)
//     }
// }
//
// impl<Statistic: PositionSummariser> RedisRepository<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned
// {
//     /// Constructs a new [`RedisRepository`] component using the provided Redis connection struct.
//     pub fn new(connection: Connection) -> Self {
//         Self { conn: connection, _statistic_marker: PhantomData::<Statistic>::default() }
//     }
//
//     /// Returns a [`RedisRepositoryBuilder`] instance.
//     pub fn builder() -> RedisRepositoryBuilder<Statistic> {
//         RedisRepositoryBuilder::new()
//     }
//
//     /// Establish & return a Redis connection.
//     pub fn setup_redis_connection(cfg: Config) -> Connection {
//         redis::Client::open(cfg.uri)
//             .expect("Failed to create Redis client")
//             .get_connection()
//             .expect("Failed to connect to Redis")
//     }
// }
//
// /// Builder to construct [`RedisRepository`] instances.
// #[derive(Default)]
// pub struct RedisRepositoryBuilder<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned
// {
//     conn: Option<Connection>,
//     _statistic_marker: PhantomData<Statistic>,
// }
//
// impl<Statistic: PositionSummariser> RedisRepositoryBuilder<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned
// {
//     pub fn new() -> Self {
//         Self {
//             conn: None,
//             _statistic_marker: PhantomData::<Statistic>::default()
//         }
//     }
//
//     pub fn conn(self, value: Connection) -> Self {
//         Self {
//             conn: Some(value),
//             ..self
//         }
//     }
//
//     pub fn build(self) -> Result<RedisRepository<Statistic>, PortfolioError> {
//         let conn = self.conn.ok_or(PortfolioError::BuilderIncomplete)?;
//
//         Ok(RedisRepository {
//             conn,
//             _statistic_marker: PhantomData::<Statistic>::default()
//         })
//     }
// }
//
// impl<Statistic> Debug for RedisRepositoryBuilder<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned
// {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("RedisRepositoryBuilder")
//             .field("conn", &"Option<redis::Connection>")
//             .field("_statistic_market", &self._statistic_marker)
//             .finish()
//     }
// }