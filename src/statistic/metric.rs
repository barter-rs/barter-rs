use crate::portfolio::position::{Position, Direction};
use serde::{Serialize, Deserialize};

pub trait MetricRolling {
    const METRIC_ID: &'static str;
    fn init() -> Self;
    fn update(&mut self, position: &Position);
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct ProfitLoss {
    pub long_contracts: f64,
    pub long_pnl: f64,
    pub long_pnl_per_contract: f64,
    pub short_contracts: f64,
    pub short_pnl: f64,
    pub short_pnl_per_contract: f64,
    pub total_contracts: f64,
    pub total_pnl: f64,
    pub total_pnl_per_contract: f64,
}

impl MetricRolling for ProfitLoss {
    const METRIC_ID: &'static str = "Profit & Loss";

    fn init() -> Self {
        Self {
            long_contracts: 0.0,
            long_pnl: 0.0,
            long_pnl_per_contract: 0.0,
            short_contracts: 0.0,
            short_pnl: 0.0,
            short_pnl_per_contract: 0.0,
            total_contracts: 0.0,
            total_pnl: 0.0,
            total_pnl_per_contract: 0.0,
        }
    }

    fn update(&mut self, position: &Position) {
        self.total_contracts += position.quantity.abs();
        self.total_pnl += position.result_profit_loss;
        self.total_pnl_per_contract = self.total_pnl / self.total_contracts;

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
    }
}