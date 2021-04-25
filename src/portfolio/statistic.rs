use std::collections::HashMap;
use crate::execution::fill::FillEvent;
use crate::portfolio::position::{Position, Direction};
use crate::portfolio::error::PortfolioError;

pub trait RollingStatistic {
    fn init(position: &Position) -> Self;
    fn update_from_position_exit(&mut self, position: &Position);
}

// Todo: Do I want to add 'strategy name' to SignalEvent, and use <strategy>_<symbol> as map key?
pub struct MetaStatistics {
    profit_loss: HashMap<String, ProfitLoss>,
    // sharpe_ratio: HashMap<String, f64>,
}

impl MetaStatistics {
    pub fn new() -> Self {
        Self {
            profit_loss: HashMap::new(),
        }
    }

    pub fn update_from_position_exit(&mut self, position: &Position) {
        match self.profit_loss.get_mut(&position.symbol) {
            None => {
                self.profit_loss.insert(position.symbol.clone(), ProfitLoss::init(position))
            },
            Some(profit_loss) => {
                profit_loss.update_from_position_exit(position)
            }
        }

        return
    }
}

/// Profit & Loss in the base currency denomination (eg/ USD).
struct ProfitLoss {
    long_contracts: f64,
    long_pnl: f64,
    long_pnl_per_contract: f64,
    short_contracts: f64,
    short_pnl: f64,
    short_pnl_per_contract: f64,
    total_contracts: f64,
    total_pnl: f64,
    total_pnl_per_contract: f64,
}

impl RollingStatistic for ProfitLoss {
    fn init(position: &Position) -> Self {
        let total_contracts = position.quantity.abs();
        let total_pnl = position.result_profit_loss;
        let total_pnl_per_contract = total_pnl / total_contracts;

        let (
            long_contracts, long_pnl, long_pnl_per_contract,
            short_contracts, short_pnl, short_pnl_per_contract
        ) = match position.direction {
            Direction::Long => {
                (total_contracts, total_pnl, total_pnl_per_contract, 0.0, 0.0, 0.0)
            }
            Direction::Short => {
                (0.0, 0.0, 0.0, total_contracts, total_pnl, total_pnl_per_contract)
            }
        };

        Self {
            long_contracts,
            long_pnl,
            long_pnl_per_contract,
            short_contracts,
            short_pnl,
            short_pnl_per_contract,
            total_contracts,
            total_pnl,
            total_pnl_per_contract
        }
    }

    fn update_from_position_exit(&mut self, position: &Position) {
        match position.direction {
            Direction::Long => {
                self.long_contracts += position.quantity.abs();
                self.long_pnl += position.result_profit_loss;
                self.long_pnl_per_contract = self.long_pnl / self.long_contracts;
            }
            Direction::Short => {
                self.short_contracts += position.quantity.abs();
                self.short_pnl += position.result_profit_loss;
                self.short_pnl_per_contract = self.short_pnl / self.short_contracts;
            }
        }

        self.total_contracts += position.quantity.abs();
        self.total_pnl += position.result_profit_loss;
        self.total_pnl_per_contract = self.total_pnl / self.total_contracts;
    }
}