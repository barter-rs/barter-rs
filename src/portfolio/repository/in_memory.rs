use crate::portfolio::position::{determine_position_id, Position, PositionId};
use crate::portfolio::repository::error::RepositoryError;
use crate::portfolio::repository::{determine_cash_id, CashHandler, PositionHandler, TotalEquity, EquityId, CashId, AvailableCash, determine_exited_positions_id, EquityHandler, determine_equity_id, StatisticHandler};
use crate::statistic::summary::PositionSummariser;
use std::collections::HashMap;
use uuid::Uuid;
use crate::{Market, MarketId};

/// In-Memory repository for Proof Of Concepts. Implements [`PositionHandler`], [`EquityHandler`],
/// [`CashHandler`] & [`StatisticHandler`]. Used by a Proof Of Concept Portfolio implementation to
/// save the current equity, available cash, Positions, and market pair statistics.
/// **Do not use in production - no fault tolerant guarantees!**
#[derive(Debug)]
pub struct InMemoryRepository<Statistic: PositionSummariser> {
    open_positions: HashMap<PositionId, Position>,
    closed_positions: HashMap<String, Vec<Position>>,
    current_equities: HashMap<EquityId, TotalEquity>,
    current_cashes: HashMap<CashId, AvailableCash>,
    statistics: HashMap<MarketId, Statistic>
}

impl<Statistic: PositionSummariser> PositionHandler for InMemoryRepository<Statistic> {
    fn set_open_position(&mut self, position: Position) -> Result<(), RepositoryError> {
        self.open_positions.insert(position.position_id.clone(), position);
        Ok(())
    }

    fn get_open_position(&mut self, position_id: &PositionId) -> Result<Option<Position>, RepositoryError> {
        Ok(self
            .open_positions
            .get(position_id)
            .map(Position::clone)
        )
    }

    fn get_open_positions<'a, Markets: Iterator<Item=&'a Market>>(&mut self, engine_id: &Uuid, markets: Markets) -> Result<Vec<Position>, RepositoryError> {
        Ok(markets
            .filter_map(|market| {
                self.open_positions
                    .get(&determine_position_id(engine_id, market.exchange, &market.symbol))
                    .map(Position::clone)
            })
            .collect()
        )
    }

    fn remove_position(&mut self, position_id: &String) -> Result<Option<Position>, RepositoryError> {
        Ok(self.open_positions.remove(position_id))
    }

    fn set_exited_position(
        &mut self,
        engine_id: &Uuid,
        position: Position,
    ) -> Result<(), RepositoryError> {
        let exited_positions_key = determine_exited_positions_id(engine_id);

        match self.closed_positions.get_mut(&exited_positions_key) {
            None => { self.closed_positions.insert(exited_positions_key, vec![position]); },
            Some(closed_positions) => closed_positions.push(position),
        }

        Ok(())
    }

    fn get_exited_positions(&mut self, engine_id: &Uuid) -> Result<Option<Vec<Position>>, RepositoryError> {
        Ok(self
            .closed_positions
            .get(&determine_exited_positions_id(engine_id))
            .map(|exited_positions| {
                exited_positions
                    .iter()
                    .map(Position::clone)
                    .collect()
            })
        )
    }
}

impl<Statistic: PositionSummariser> EquityHandler for InMemoryRepository<Statistic> {
    fn set_total_equity(&mut self, engine_id: &Uuid, total_equity: TotalEquity) -> Result<(), RepositoryError> {
        self.current_equities.insert(determine_equity_id(engine_id), total_equity);
        Ok(())
    }

    fn get_total_equity(&mut self, engine_id: &Uuid) -> Result<TotalEquity, RepositoryError> {
        match self.current_equities.get(&determine_equity_id(engine_id)) {
            None => Err(RepositoryError::ExpectedDataNotPresentError),
            Some(value) => Ok(*value),
        }
    }
}

impl<Statistic: PositionSummariser> CashHandler for InMemoryRepository<Statistic> {
    fn set_available_cash(&mut self, engine_id: &Uuid, cash: AvailableCash) -> Result<(), RepositoryError> {
        self.current_cashes.insert(determine_cash_id(engine_id), cash);
        Ok(())
    }

    fn get_available_cash(&mut self, engine_id: &Uuid) -> Result<AvailableCash, RepositoryError> {
        match self.current_cashes.get(&determine_cash_id(engine_id)) {
            None => Err(RepositoryError::ExpectedDataNotPresentError),
            Some(cash) => Ok(*cash),
        }
    }
}

impl<Statistic: PositionSummariser> StatisticHandler<Statistic> for InMemoryRepository<Statistic> {
    fn set_statistics(&mut self, market_id: &MarketId, statistic: Statistic) -> Result<(), RepositoryError> {
        self.statistics.insert(market_id.clone(), statistic);
        Ok(())
    }

    fn get_statistics(&mut self, market_id: &MarketId) -> Result<Statistic, RepositoryError> {
        match self.statistics.get(market_id) {
            None => Err(RepositoryError::ExpectedDataNotPresentError),
            Some(statistics) => Ok(*statistics)
        }
    }
}

impl<Statistic: PositionSummariser> InMemoryRepository<Statistic> {
    /// Constructs a new [InMemoryRepository] component.
    pub fn new() -> Self {
        Self {
            open_positions: HashMap::new(),
            closed_positions: HashMap::new(),
            current_equities: HashMap::with_capacity(1),
            current_cashes: HashMap::with_capacity(1),
            statistics: HashMap::new(),
        }
    }
}