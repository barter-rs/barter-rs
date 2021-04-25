use crate::portfolio::position::{Position, Direction};

// long count, short count, long average %, short average %,
// average profit %, cum long %, cum short %, cum profit %, total profit <denomination>, avg duration trade
// getOpenPositions() to aid backtest/trade performance by auto closing open positions

/// Profit & Loss metrics in the base currency denomination (eg/ USD).
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

impl ProfitLoss {
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

    fn next(&self, position: &Position) -> Self {
        let next_total_contracts = self.total_contracts + position.quantity.abs();
        let next_total_pnl = self.total_pnl + position.result_profit_loss;

        let (
            next_long_contracts, next_long_pnl, next_long_pnl_per_contract,
            next_short_contracts, next_short_pnl, next_short_pnl_per_contract
        ) = match position.direction {
            Direction::Long => {
                let next_long_contracts = self.long_contracts + position.quantity.abs();
                let next_long_pnl = self.long_pnl + position.result_profit_loss;
                (
                    next_long_contracts, next_long_pnl, (next_long_pnl / next_long_contracts),
                    self.short_contracts, self.short_pnl, self.short_pnl_per_contract
                )
            }
            Direction::Short => {
                let next_short_contracts = self.short_contracts + position.quantity.abs();
                let next_short_pnl = self.short_pnl + position.result_profit_loss;
                (
                    self.long_contracts, self.long_pnl, self.long_pnl_per_contract,
                    next_short_contracts, next_short_pnl, (next_short_pnl / next_short_contracts)
                )
            }
        };

        Self {
            long_contracts: next_long_contracts,
            long_pnl: next_long_pnl,
            long_pnl_per_contract: next_long_pnl_per_contract,
            short_contracts: next_short_contracts,
            short_pnl: next_short_pnl,
            short_pnl_per_contract: next_short_pnl_per_contract,
            total_contracts: next_total_contracts,
            total_pnl: next_total_pnl,
            total_pnl_per_contract: (next_total_pnl / next_total_contracts)
        }
    }
}