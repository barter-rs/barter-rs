use crate::portfolio::position::Position;
use crate::statistic::metric::{MetricRolling, ProfitLoss};
use std::collections::HashMap;
use std::fmt::Display;
use prettytable::{Row, Table};
use crate::statistic::error::StatisticError;

// Todo:
//  - &'static associated constants?
//  - Remove duplicated generate_statistics() method for pnl_sheet & tear_sheet -> interface? Delete all together?

pub trait Summariser {
    const SUMMARY_ID: &'static str;
    fn update_summary(&mut self, position: &Position);
    fn print_table(&self);
}

pub struct TradingStatistics<T> where T: MetricRolling {
    pnl_sheet: ProfitLossSheet,
    tear_sheet: TearSheet<T>
}

impl<T> TradingStatistics<T> where T: MetricRolling + Display + Clone {
    pub fn new(pnl_sheet: ProfitLossSheet, tear_sheet: TearSheet<T>) -> Self {
        Self {
            pnl_sheet,
            tear_sheet,
        }
    }

    pub fn generate_statistics(&mut self, positions: &Vec<Position>) {
        for position in positions.iter() {
            self.pnl_sheet.update_summary(position);
            self.tear_sheet.update_summary(position);
        }
    }

    pub fn print_statistics(&self) {
        println!("\n-- Profit & Loss Sheet --");
        self.pnl_sheet.print_table();
        println!("\n-- Tear Sheet --");
        self.tear_sheet.print_table();
    }
}

pub type SymbolID = String;
pub type MetricID = String;

pub struct ProfitLossSheet {
    sheet: HashMap<SymbolID, ProfitLoss>,
}

impl Summariser for ProfitLossSheet {
    const SUMMARY_ID: &'static str = "Profit & Loss Sheet";

    fn update_summary(&mut self, position: &Position) {
        match self.sheet.get_mut(&*position.symbol) {
            None => {
                let mut first_symbol_pnl = ProfitLoss::init();
                first_symbol_pnl.update(position);
                self.sheet.insert(position.symbol.clone(), first_symbol_pnl);
            }
            Some(symbol_pnl) => {
                symbol_pnl.update(position);
            }
        }
    }

    fn print_table(&self) {
        let mut pnl_sheet = Table::new();

        pnl_sheet.set_titles(row!["",
            "Long Contracts", "Long PnL", "Long PnL Per Contract",
            "Short Contracts", "Short PnL", "Short PnL Per Contract",
            "Total Contracts", "Total PnL", "Total PnL Per Contract"
        ]);

        for (symbol, pnl_summary) in self.sheet.iter() {
            pnl_sheet.add_row(row![symbol,
                pnl_summary.long_contracts, pnl_summary.long_pnl, pnl_summary.long_pnl_per_contract,
                pnl_summary.short_contracts, pnl_summary.short_pnl, pnl_summary.short_pnl_per_contract,
                pnl_summary.total_contracts, pnl_summary.total_pnl, pnl_summary.total_pnl_per_contract
            ]);
        }

        pnl_sheet.printstd();
    }
}

impl ProfitLossSheet {
    pub fn new() -> Self {
        Self {
            sheet: HashMap::<SymbolID, ProfitLoss>::new()
        }
    }

    pub fn generate_statistics(&mut self, positions: &Vec<Position>) {
        for position in positions {
            self.update_summary(position);
        }
    }
}

pub struct TearSheet<T> where T: MetricRolling {
    init_metrics: HashMap<MetricID, T>,
    sheet: HashMap<SymbolID, HashMap<MetricID, T>>,
}

impl<T> Summariser for TearSheet<T> where T: MetricRolling + Display + Clone {
    const SUMMARY_ID: &'static str = "Tear Sheet";

    fn update_summary(&mut self, position: &Position) {
        match self.sheet.get_mut(&*position.symbol) {
            None => {
                let mut first_symbol_metrics = self.init_metrics.clone();

                first_symbol_metrics
                    .values_mut()
                    .map(|metric| T::update(metric, position))
                    .next();

                self.sheet.insert(position.symbol.clone(), first_symbol_metrics);
            }

            Some(symbol_metrics) => {
                symbol_metrics
                    .values_mut()
                    .map(|metric| T::update(metric, position))
                    .next();
            }
        }
    }

    fn print_table(&self) {
        let mut tear_sheet = Table::new();
        let mut titles = vec![""];

        for (symbol, metrics) in self.sheet.iter() {
            for (metric, value) in metrics.iter() {
                titles.push(metric);
                tear_sheet.add_row(row![symbol, value]);

            }
        }

        tear_sheet.set_titles(Row::from(titles));
        tear_sheet.printstd();
    }
}

impl<T> TearSheet<T> where T: MetricRolling + Display + Clone {
    pub fn new() -> Self {
        Self {
            init_metrics: HashMap::<MetricID, T>::new(),
            sheet: HashMap::<SymbolID, HashMap<MetricID, T>>::new()
        }
    }

    pub fn builder() -> TearSheetBuilder<T> {
        TearSheetBuilder::new()
    }

    pub fn generate_statistics(&mut self, positions: &Vec<Position>) {
        for position in positions {
            self.update_summary(position);
        }
    }
}

pub struct TearSheetBuilder<T> where T: MetricRolling {
    metrics: HashMap<MetricID, T>,
}

impl<T> TearSheetBuilder<T> where T: MetricRolling + Clone {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::<String, T>::new(),
        }
    }

    pub fn metric(mut self, metric: T) -> Self {
        // Return without modification if metric name already exists in metrics Map
        if self.metrics.contains_key(&*T::METRIC_ID) {
            return self
        }

        // Insert new metric initial value into Map
        self.metrics.insert(String::from(T::METRIC_ID), metric);
        self
    }

    pub fn build(self) -> Result<TearSheet<T>, StatisticError> {
        match self.metrics.is_empty() {
            true => {
                Err(StatisticError::BuilderNoMetricsProvided)
            }
            false => {
                Ok(TearSheet {
                    init_metrics: self.metrics,
                    sheet: HashMap::<SymbolID, HashMap<MetricID, T>>::new(),
                })
            }
        }
    }
}