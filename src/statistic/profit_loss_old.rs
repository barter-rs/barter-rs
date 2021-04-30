// use crate::portfolio::position::{Position, Direction};
// use serde::{Deserialize, Serialize};
// use crate::statistic::metric::{MetricRolling, MetricInitialiser, MetricTimeSeries, MetricSummariser};

// /// Profit & Loss metrics in the base currency denomination (eg/ USD).
// #[derive(Debug, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
// pub struct ProfitLoss {
//     pub long_contracts: f64,
//     pub long_pnl: f64,
//     pub long_pnl_per_contract: f64,
//     pub short_contracts: f64,
//     pub short_pnl: f64,
//     pub short_pnl_per_contract: f64,
//     pub total_contracts: f64,
//     pub total_pnl: f64,
//     pub total_pnl_per_contract: f64,
// }
//
// impl MetricInitialiser for ProfitLoss {
//     fn init() -> Self {
//         Self {
//             long_contracts: 0.0,
//             long_pnl: 0.0,
//             long_pnl_per_contract: 0.0,
//             short_contracts: 0.0,
//             short_pnl: 0.0,
//             short_pnl_per_contract: 0.0,
//             total_contracts: 0.0,
//             total_pnl: 0.0,
//             total_pnl_per_contract: 0.0,
//         }
//     }
// }
//
// impl MetricRolling for ProfitLoss {
//     const METRIC_ID: String = String::from("PnL Summary");
//
//     fn update(&mut self, position: &Position) {
//         self.total_contracts += position.quantity.abs();
//         self.total_pnl += position.result_profit_loss;
//         self.total_pnl_per_contract = self.total_pnl / self.total_contracts;
//
//         match position.direction {
//             Direction::Long => {
//                 self.long_contracts += position.quantity.abs();
//                 self.long_pnl += position.result_profit_loss;
//                 self.long_pnl_per_contract = self.long_pnl / self.long_contracts;
//             }
//             Direction::Short => {
//                 self.short_contracts += position.quantity.abs();
//                 self.short_pnl += position.result_profit_loss;
//                 self.short_pnl_per_contract = self.short_pnl / self.short_contracts;
//             }
//         }
//     }
// }

// impl MetricTimeSeries for ProfitLoss {
//     fn generate_next(&self, position: &Position) -> ProfitLoss {
//         let next_total_contracts = self.total_contracts + position.quantity.abs();
//         let next_total_pnl = self.total_pnl + position.result_profit_loss;
//
//         let (
//             next_long_contracts, next_long_pnl, next_long_pnl_per_contract,
//             next_short_contracts, next_short_pnl, next_short_pnl_per_contract
//         ) = match position.direction {
//             Direction::Long => {
//                 let next_long_contracts = self.long_contracts + position.quantity.abs();
//                 let next_long_pnl = self.long_pnl + position.result_profit_loss;
//                 (
//                     next_long_contracts, next_long_pnl, (next_long_pnl / next_long_contracts),
//                     self.short_contracts, self.short_pnl, self.short_pnl_per_contract
//                 )
//             }
//             Direction::Short => {
//                 let next_short_contracts = self.short_contracts + position.quantity.abs();
//                 let next_short_pnl = self.short_pnl + position.result_profit_loss;
//                 (
//                     self.long_contracts, self.long_pnl, self.long_pnl_per_contract,
//                     next_short_contracts, next_short_pnl, (next_short_pnl / next_short_contracts)
//                 )
//             }
//         };
//
//         Self {
//             long_contracts: next_long_contracts,
//             long_pnl: next_long_pnl,
//             long_pnl_per_contract: next_long_pnl_per_contract,
//             short_contracts: next_short_contracts,
//             short_pnl: next_short_pnl,
//             short_pnl_per_contract: next_short_pnl_per_contract,
//             total_contracts: next_total_contracts,
//             total_pnl: next_total_pnl,
//             total_pnl_per_contract: (next_total_pnl / next_total_contracts)
//         }
//     }
// }