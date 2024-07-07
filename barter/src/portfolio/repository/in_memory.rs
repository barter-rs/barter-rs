use crate::{
    portfolio::{
        position::{determine_position_id, Position, PositionId},
        repository::{
            determine_exited_positions_id, error::RepositoryError, BalanceHandler, PositionHandler,
            StatisticHandler,
        },
        Balance, BalanceId,
    },
    statistic::summary::PositionSummariser,
};
use barter_integration::model::{Market, MarketId};
use std::collections::HashMap;
use uuid::Uuid;

/// In-Memory repository for Proof Of Concepts. Implements [`PositionHandler`], [`BalanceHandler`]
/// & [`StatisticHandler`]. Used by a Proof Of Concept Portfolio implementation to
/// save the current equity, available cash, Positions, and market pair statistics.
/// **Careful in production - no fault tolerant guarantees!**
#[derive(Debug, Default)]
pub struct InMemoryRepository<Statistic: PositionSummariser> {
    open_positions: HashMap<PositionId, Position>,
    closed_positions: HashMap<String, Vec<Position>>,
    current_balances: HashMap<BalanceId, Balance>,
    statistics: HashMap<MarketId, Statistic>,
}

impl<Statistic: PositionSummariser> PositionHandler for InMemoryRepository<Statistic> {
    fn set_open_position(&mut self, position: Position) -> Result<(), RepositoryError> {
        self.open_positions
            .insert(position.position_id.clone(), position);
        Ok(())
    }

    fn get_open_position(
        &mut self,
        position_id: &PositionId,
    ) -> Result<Option<Position>, RepositoryError> {
        Ok(self.open_positions.get(position_id).map(Position::clone))
    }

    fn get_open_positions<'a, Markets: Iterator<Item = &'a Market>>(
        &mut self,
        engine_id: Uuid,
        markets: Markets,
    ) -> Result<Vec<Position>, RepositoryError> {
        Ok(markets
            .filter_map(|market| {
                self.open_positions
                    .get(&determine_position_id(
                        engine_id,
                        &market.exchange,
                        &market.instrument,
                    ))
                    .map(Position::clone)
            })
            .collect())
    }

    fn remove_position(
        &mut self,
        position_id: &String,
    ) -> Result<Option<Position>, RepositoryError> {
        Ok(self.open_positions.remove(position_id))
    }

    fn set_exited_position(
        &mut self,
        engine_id: Uuid,
        position: Position,
    ) -> Result<(), RepositoryError> {
        let exited_positions_key = determine_exited_positions_id(engine_id);

        match self.closed_positions.get_mut(&exited_positions_key) {
            None => {
                self.closed_positions
                    .insert(exited_positions_key, vec![position]);
            }
            Some(closed_positions) => closed_positions.push(position),
        }
        Ok(())
    }

    fn get_exited_positions(&mut self, engine_id: Uuid) -> Result<Vec<Position>, RepositoryError> {
        Ok(self
            .closed_positions
            .get(&determine_exited_positions_id(engine_id))
            .map(Vec::clone)
            .unwrap_or_else(Vec::new))
    }
}

impl<Statistic: PositionSummariser> BalanceHandler for InMemoryRepository<Statistic> {
    fn set_balance(&mut self, engine_id: Uuid, balance: Balance) -> Result<(), RepositoryError> {
        self.current_balances
            .insert(Balance::balance_id(engine_id), balance);
        Ok(())
    }

    fn get_balance(&mut self, engine_id: Uuid) -> Result<Balance, RepositoryError> {
        self.current_balances
            .get(&Balance::balance_id(engine_id))
            .copied()
            .ok_or(RepositoryError::ExpectedDataNotPresentError)
    }
}

impl<Statistic: PositionSummariser> StatisticHandler<Statistic> for InMemoryRepository<Statistic> {
    fn set_statistics(
        &mut self,
        market_id: MarketId,
        statistic: Statistic,
    ) -> Result<(), RepositoryError> {
        self.statistics.insert(market_id, statistic);
        Ok(())
    }

    fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError> {
        self.statistics
            .get(market_id)
            .copied()
            .ok_or(RepositoryError::ExpectedDataNotPresentError)
    }
}

impl<Statistic: PositionSummariser> InMemoryRepository<Statistic> {
    /// Constructs a new [`InMemoryRepository`] component.
    pub fn new() -> Self {
        Self {
            open_positions: HashMap::new(),
            closed_positions: HashMap::new(),
            current_balances: HashMap::new(),
            statistics: HashMap::new(),
        }
    }
}
