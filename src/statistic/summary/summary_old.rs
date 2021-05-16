// use crate::portfolio::position::Position;
// use crate::statistic::metric::ratio::{SharpeRatio, SortinoRatio, Ratio};
// use crate::statistic::metric::sharpe_ratio_spike::PnLReturnSummary;
// use crate::statistic::summary::summary_older::SummariserOld;
// use prettytable::{Row, Table};
// use serde::Deserialize;
// use crate::statistic::metric::profit_loss::MetricRolling;
//
// // Todo: Add Mean trait? And/or other calculation traits, similar to std::op::add

// #[derive(Debug, Deserialize)]
// pub struct Config {
//     pub trading_days_per_year: usize,
//     pub risk_free_return: f64,
// }
//
// #[derive(Debug, Clone, PartialOrd, PartialEq)]
// pub struct TradingSummary {
//     pnl_returns: PnLReturnSummary,
//     tear_sheet: TearSheet,
// }
//
// impl PositionSummariser for TradingSummary {
//     fn update(&mut self, position: &Position) {
//         self.pnl_returns.update_summary(position);
//
//         println!("\nDirection: {:?}, Quantity: {:?}, Enter: {:?}, Exit: {:?}, PnL: {:?}, Return: {:?}, Mean: {:?}, Std. Dev: {:?}, Count: {:?}, Duration Secs: {:?}",
//                  position.direction,
//                  position.quantity,
//                  position.enter_value_gross,
//                  position.exit_value_gross,
//                  position.result_profit_loss,
//                  position.calculate_profit_loss_return(),
//                  self.pnl_returns.total.mean,
//                  self.pnl_returns.total.dispersion.std_dev,
//                  self.pnl_returns.total.count,
//                  self.pnl_returns.duration.num_seconds(),
//         );
//
//         self.tear_sheet.update(position, &self.pnl_returns);
//         println!("Sharpe Per Trade: {:?}, Sharpe Annual: {:?}, Sharpe Daily: {:?}",
//                  self.tear_sheet.sharpe_ratio.sharpe_ratio_per_trade,
//                  self.tear_sheet.sharpe_ratio.annual(365),
//                 self.tear_sheet.sharpe_ratio.daily()
//         );
//     }
//
//     fn print(&self) {
//         println!("\n-- Tear Sheet --");
//         self.tear_sheet.print_table();
//     }
// }
//
// impl TradingSummary {
//     pub fn new(cfg: &Config) -> Self {
//         Self {
//             pnl_returns: PnLReturnSummary::init(),
//             tear_sheet: TearSheet::new(cfg.risk_free_return)
//         }
//     }
// }

// #[derive(Debug, Clone, PartialOrd, PartialEq)]
// pub struct TearSheet {
//     // drawdown: Drawdown,
//     sharpe_ratio: SharpeRatio,
//     sortino_ratio: SortinoRatio,
//     // calmar_ratio: CalmarRatio,
// }
//
// impl TearSheet {
//     pub fn new(risk_free_return: f64) -> Self {
//         Self {
//             // drawdown: Drawdown::init(),
//             sharpe_ratio: SharpeRatio::init(risk_free_return),
//             sortino_ratio: SortinoRatio::init(risk_free_return),
//             // calmar_ratio: CalmarRatio::init(risk_free_return),
//         }
//     }
//
//     pub fn update(&mut self, position: &Position, pnl_return_view: &PnLReturnSummary) {
//         // self.drawdown.update(position);
//         self.sharpe_ratio.update(pnl_return_view);
//         self.sortino_ratio.update(pnl_return_view);
//         // self.calmar_ratio.update(pnl_return_view, self.drawdown.max_drawdown);
//     }
// }
//
// impl TablePrinter for TearSheet {
//     fn print_table(&self) {
//         let mut tear_sheet = Table::new();
//
//         // let titles = vec!["",
//         //                   "Avg. Drawdown", "Avg. Drawdown Duration", "Max Drawdown", "Max Drawdown Duration",
//         //                   "Sharpe Ratio", "Sortino Ratio", "Calmar Ratio"
//         // ];
//
//         let titles = vec!["", "Sharpe Ratio", "Sortino Ratio"];
//
//         tear_sheet.add_row(row!["Total",
//             // self.drawdown.avg_drawdown, self.drawdown.avg_drawdown_duration.to_string(),
//             // self.drawdown.max_drawdown, self.drawdown.max_drawdown_duration.to_string(),
//             self.sharpe_ratio.daily().to_string(),
//             self.sortino_ratio.daily().to_string(),
//             // self.calmar_ratio.calculate_daily().to_string(),
//         ]);
//
//         tear_sheet.set_titles(Row::from(titles));
//         tear_sheet.printstd();
//     }
// }