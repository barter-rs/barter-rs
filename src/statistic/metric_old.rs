// use std::collections::HashMap;
// use crate::portfolio::position::Position;
// use crate::statistic::error::StatisticError;

// pub trait MetricInitialiser {
//     fn init() -> Self;
// }
//
// pub trait MetricRolling {
//     const METRIC_ID: String;
//     fn update(&mut self, position: &Position);
// }
//
// pub trait Summary {
//     const SUMMARY_ID: &'static str;
//     fn print_table(&self);
// }
//
// pub type SymbolID = String;
// pub type MetricID = String;
//
// pub struct PnLSummaryOld<T>(HashMap<SymbolID, T>) where T: MetricRolling;
//
// impl<T> Summary for PnLSummaryOld<T> where T: MetricRolling {
//     const SUMMARY_ID: &'static str = "PnL Summary";
//     fn print_table(&self) {
//         // Todo:
//     }
// }
//
// pub struct TearSheetOld<T>(HashMap<SymbolID, HashMap<MetricID, T>>) where T: MetricRolling;
//
// impl<T> Summary for TearSheetOld<T> where T: MetricRolling {
//     const SUMMARY_ID: &'static str = "Tear Sheet";
//     fn print_table(&self) {
//         // Todo:
//     }
// }
//
// pub struct TradingStatisticsOld<T> where T: MetricRolling {
//     summary: PnLSummaryOld<T>,
//     tear_sheet: Option<TearSheetOld<T>>,
//     // summary: HashMap<SymbolID, T>,
//     // tear_sheet: Option<HashMap<SymbolID, HashMap<MetricID, T>>>,
// }
//
// impl<T> TradingStatisticsOld<T> where T: MetricRolling + Clone {
//     pub fn new(summary: PnLSummaryOld<T>, tear_sheet: Option<TearSheetOld<T>>) -> Self {
//         Self {
//             summary,
//             tear_sheet
//         }
//     }
//
//     pub fn builder() -> TradingStatisticsBuilder<T> {
//         TradingStatisticsBuilder::new()
//     }
//
//     pub fn generate_summary(symbols: &Vec<SymbolID>, closed_positions: &Vec<Position>,
//                             summary_init_value: T) -> HashMap<SymbolID, T> {
//         let mut summary = HashMap::with_capacity(symbols.len());
//
//         for position in closed_positions.iter() {
//             match summary.get_mut(&*position.symbol) {
//                 None => {
//                     summary.insert(position.symbol.clone(), summary_init_value.clone());
//                 }
//                 Some(symbol_summary) => {
//                     symbol_summary.update(position)
//                 }
//             }
//         }
//
//         summary
//     }
// }

// pub struct TradingStatisticsBuilder<T> where T: MetricRolling + Clone {
//     symbols: Option<Vec<String>>,
//     closed_positions: Option<Vec<Position>>,
//     summary_init_value: Option<T>,
//     summary_generation_func: Option<fn(&Vec<SymbolID>, &Vec<Position>, T) -> PnLSummaryOld<T>>,
//     metrics: HashMap<MetricID, T>,
// }
//
// impl<T> TradingStatisticsBuilder<T> where T: MetricRolling + Clone {
//     pub fn new() -> Self {
//         Self {
//             symbols: None,
//             closed_positions: None,
//             summary_init_value: None,
//             summary_generation_func: None,
//             metrics: HashMap::<String, T>::new(),
//         }
//     }
//
//     pub fn symbols(mut self, symbols: Vec<String>) -> Self {
//         self.symbols = Some(symbols);
//         self
//     }
//
//     pub fn closed_positions(mut self, closed_position: Vec<Position>) -> Self {
//         self.closed_positions = Some(closed_position);
//         self
//     }
//
//     pub fn summary(mut self, summary_init: T, func: fn(&Vec<SymbolID>, &Vec<Position>, T) -> PnLSummaryOld<T>) -> Self {
//         self.summary_init_value = Some(summary_init);
//         // Todo: Do I even need this if it's T... just re-use logic from elsewhere & construct in same loop
//         self.summary_generation_func = Some(func);
//         self
//     }
//
//     /// Idempotent
//     pub fn statistic(mut self, metric: T) -> Self {
//         // Return without modification if metric name already exists in metrics Map
//         if self.metrics.contains_key(&*T::METRIC_ID) {
//             return self
//         }
//
//         // Insert new metric initial value into Map
//         self.metrics.insert(T::METRIC_ID, metric);
//         self
//     }
//
//     pub fn build(self) -> Result<TradingStatisticsOld<T>, StatisticError> {
//         if let (
//             Some(symbols),
//             Some(closed_positions),
//             Some(summary_init_value),
//             Some(summary_generation_func),
//         ) = (
//             self.symbols,
//             self.closed_positions,
//             self.summary_init_value,
//             self.summary_generation_func,
//         ) {
//             // Generate summary
//             let summary = summary_generation_func(
//                 &symbols, &closed_positions, summary_init_value,
//             );
//
//             // Generate tear_sheet
//             let tear_sheet = match self.metrics.is_empty() {
//                 true => None,
//                 false => {
//                     // Generate...
//                     None
//                 }
//             };
//
//             Ok(TradingStatisticsOld {
//                 summary,
//                 tear_sheet,
//             })
//         } else {
//             Err(StatisticError::BuilderIncomplete)
//         }
//     }
// }




// pub struct StatisticsRollingBuilder<T> where T: MetricRolling + Clone {
//     symbols: Option<Vec<String>>,
//     closed_positions: Option<Vec<Position>>,
//     statistic_counter: usize,
//     statistics: HashMap<String, T>,
// }
//
// // Todo - add constraints:
// //  - Ensure this is the initial statistic value!
//
// impl<T> StatisticsRollingBuilder<T> where T: MetricRolling + Clone {
//     pub fn new() -> Self {
//         Self {
//             symbols: None,
//             closed_positions: None,
//             statistic_counter: 0,
//             statistics: HashMap::<String, T>::new(),
//         }
//     }
//
//     pub fn symbols(mut self, symbols: Vec<String>) -> Self {
//         self.symbols = Some(symbols);
//         self
//     }
//
//     pub fn closed_positions(mut self, closed_position: Vec<Position>) -> Self {
//         self.closed_positions = Some(closed_position);
//         self
//     }
//
//     pub fn statistic(mut self, statistic: T) -> Self {
//         // Determine the name of the statistic type for use as a Map key
//         let statistic_id = type_name::<T>();
//
//         // Return without modification if statistic_id already statistics Map
//         if self.statistics.contains_key(statistic_id) {
//             return self
//         }
//
//         // Insert new statistic initial value into Map & increment statistic counter
//         self.statistics.insert(String::from(statistic_id), statistic);
//         self.statistic_counter += 1;
//         self
//     }
//
//     pub fn build(self) -> Result<StatisticsRolling<T>, StatisticError> {
//         if let (
//             Some(symbols),
//             Some(closed_positions)
//         ) = (
//             self.symbols,
//             self.closed_positions
//         ) {
//             // Validate non-zero statistics have been added
//             if self.statistic_counter > 0 {
//
//                 // Allocate HashMap memory for every symbol
//                 let mut symbol_statistics = HashMap::with_capacity(symbols.len());
//
//                 // Construct HashMap Entry for every symbol
//                 for symbol in symbols {
//                     // symbol_statistics.insert(symbol, self.statistics.clone());
//                     symbol_statistics.insert(symbol, self.statistics.clone());
//                 }
//
//                 // Generate statistics
//                 symbol_statistics = StatisticsRollingBuilder::generate_statistics(
//                     symbol_statistics, closed_positions);
//
//                 Ok(StatisticsRolling {
//                     statistics: symbol_statistics,
//                 })
//
//             } else {
//                 Err(StatisticError::BuilderNoStatisticsProvided)
//             }
//         } else {
//             Err(StatisticError::BuilderIncomplete)
//         }
//     }
//
//     fn generate_statistics(mut symbol_statistics: HashMap<String, HashMap<String, T>>,
//                            closed_position: Vec<Position>) -> HashMap<String, HashMap<String, T>> {
//         // Loop through Positions
//         // Identify symbol Position is associated with
//         // Retrieve that symbol's stats from the Map to return HashMap<Statistic, T>
//         // Update T w/ &Position
//
//         for position in closed_position.into_iter() {
//             symbol_statistics
//                 .entry(position.symbol.clone())
//                 .and_modify(
//                     |statistics| {
//
//                         statistics
//                             .values_mut()
//                             .map(|stat| {
//                                 stat.update(&position)
//                             })
//                             .next();
//
//                     }
//                 );
//         }
//
//         symbol_statistics
//     }
// }
// impl<T> StatisticsRolling<T> where T: MetricRolling + Clone {
//     pub fn builder() -> StatisticsRollingBuilder<T> {
//         StatisticsRollingBuilder::new()
//     }
//
//     pub fn print(&self) {
//         // Create the table
//         let mut table = Table::new();
//
//         table.set_titles(row!["", "Stat1"]);
//         table.add_row(row!["ETH-USD", 50]);
//         table.add_row(row!["ETH-USD", 50]);
//
//         table.printstd();
//
//         let mut titles = Vec::new();
//
//         for (symbol, statistics) in self.statistics.iter() {
//
//             for (statistic, value) in statistics.iter() {
//
//                 titles.push(statistic)
//
//             }r
//         }
//     }
// }
// pub struct StatisticsRolling<T> where T: MetricRolling {
//     // HashMap<symbol, HashMap<statistic_id, value>>
//     statistics: HashMap<String, HashMap<String, T>>,
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistic::profit_loss::ProfitLoss;

    #[test]
    fn build_stats() {
        let mut position = Position::default();
        position.result_profit_loss = 100.0;

        let symbol = position.symbol.clone();

        let closed_positions = vec![position];
        let symbols = vec![symbol];

        let statistics = StatisticsRolling::builder()
            .symbols(symbols)
            .closed_positions(closed_positions)
            .statistic(ProfitLoss::init())
            .build().unwrap();

    }
}