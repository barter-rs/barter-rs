use std::collections::HashMap;
use crate::portfolio::position::Position;
use crate::statistic::error::StatisticError;
use std::any::type_name;

pub trait StatisticInitialiser {
    fn init() -> Self;
}

pub trait StatisticRolling {
    fn update(&mut self, position: &Position);
    fn print(&self);
}

pub trait StatisticTimeSeries {
    fn generate_next(&self, position: &Position) -> Self;
}

pub trait StatisticsGenerator {
    fn generate_historic_statistics(&self, closed_positions: Vec<Position>);
}

pub struct StatisticsRolling<T> where T: StatisticRolling {
    // HashMap<symbol, HashMap<statistic_id, value>>
    statistics: HashMap<String, HashMap<String, T>>,
}

impl<T> StatisticsRolling<T> where T: StatisticRolling + Clone {
    pub fn builder() -> StatisticsRollingBuilder<T> {
        StatisticsRollingBuilder::new()
    }
}

pub struct StatisticsRollingBuilder<T> where T: StatisticRolling + Clone {
    symbols: Option<Vec<String>>,
    closed_positions: Option<Vec<Position>>,
    statistic_counter: usize,
    statistics: HashMap<String, T>,
}

// Todo - add constraints:
//  - Ensure this is the initial statistic value!
//  - Ensure that type_name returns the concrete impl!
//  - Find a way for users to generate statistics at the end

impl<T> StatisticsRollingBuilder<T> where T: StatisticRolling + Clone {
    pub fn new() -> Self {
        Self {
            symbols: None,
            closed_positions: None,
            statistic_counter: 0,
            statistics: HashMap::<String, T>::new(),
        }
    }

    pub fn symbols(mut self, symbols: Vec<String>) -> Self {
        self.symbols = Some(symbols);
        self
    }

    pub fn closed_positions(mut self, closed_position: Vec<Position>) -> Self {
        self.closed_positions = Some(closed_position);
        self
    }

    /// Idempotent
    pub fn statistic(mut self, statistic: T) -> Self {
        // Determine the name of the statistic type for use as a Map key
        let statistic_id = type_name::<T>();

        // Return without modification if statistic_id already statistics Map
        if self.statistics.contains_key(statistic_id) {
            return self
        }

        // Insert new statistic initial value into Map & increment statistic counter
        self.statistics.insert(String::from(statistic_id), statistic);
        self.statistic_counter += 1;
        self
    }

    pub fn build(self) -> Result<StatisticsRolling<T>, StatisticError> {
        if let (
            Some(symbols),
            Some(closed_positions)
        ) = (
            self.symbols,
            self.closed_positions
        ) {
            // Validate non-zero statistics have been added
            if self.statistic_counter > 0 {

                // Allocate HashMap memory for every symbol
                let mut symbol_statistics = HashMap::with_capacity(symbols.len());

                // Construct HashMap Entry for every symbol
                for symbol in symbols {
                    // symbol_statistics.insert(symbol, self.statistics.clone());
                    symbol_statistics.insert(symbol, self.statistics.clone());
                }

                // Generate statistics
                symbol_statistics = StatisticsRollingBuilder::generate_statistics(
                    symbol_statistics, closed_positions);

                Ok(StatisticsRolling {
                    statistics: symbol_statistics,
                })
                
            } else {
                Err(StatisticError::BuilderNoStatisticsProvided)
            }
        } else {
            Err(StatisticError::BuilderIncomplete)
        }
    }

    fn generate_statistics(mut symbol_statistics: HashMap<String, HashMap<String, T>>,
                           closed_position: Vec<Position>) -> HashMap<String, HashMap<String, T>> {
        // Loop through Positions
        // Identify symbol Position is associated with
        // Retrieve that symbol's stats from the Map to return HashMap<Statistic, T>
        // Update T w/ &Position

        for position in closed_position.into_iter() {
            symbol_statistics
                .entry(position.symbol.clone())
                .and_modify(
                    |statistics| {

                        statistics
                            .values_mut()
                            .map(|stat| {
                                stat.print();
                                stat.update(&position)
                            })
                            .next();

                    }
                );
        }

        symbol_statistics
    }
}

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